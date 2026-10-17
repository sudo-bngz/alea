//! Tile culling: what a field can be over a small rectangle of the canvas.
//!
//! Before a tile of pixels is computed, the field is simplified for that tile:
//! every part whose value is proven to be the same over the whole tile (a
//! train far away from it is exactly 0 there; a shape far inside is exactly 1)
//! becomes a number. Proofs use interval arithmetic: each node gets a range
//! `[lo, hi]` that its value cannot leave. A part is replaced only when its
//! range is a single number, which the full computation would give too; the
//! pixels are the same, only faster.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

use crate::ast::BinOp;
use crate::eval::{apply_bin_pub as apply_bin, apply_builtin_pub as apply_builtin, Builtin, Event, Field};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}

const ANY: Iv = Iv { lo: f64::NEG_INFINITY, hi: f64::INFINITY };
const UNIT: Iv = Iv { lo: 0.0, hi: 1.0 };

impl Iv {
    pub fn point(v: f64) -> Iv {
        if v.is_nan() { ANY } else { Iv { lo: v, hi: v } }
    }
    fn new(lo: f64, hi: f64) -> Iv {
        if lo.is_nan() || hi.is_nan() || lo > hi { ANY } else { Iv { lo, hi } }
    }
    /// The single value, if the range is one finite number.
    pub fn single(self) -> Option<f64> {
        (self.lo == self.hi && self.lo.is_finite()).then_some(self.lo)
    }
    fn finite(self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }
    fn contains0(self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }
    /// Rounding can move a computed bound by a hair: widen inexact results a
    /// little so that no pixel is ever outside its proven range.
    fn widen(self) -> Iv {
        if self.lo == self.hi {
            return self;
        }
        let e = |v: f64| v.abs() * 1e-12 + 1e-300;
        Iv::new(self.lo - e(self.lo), self.hi + e(self.hi))
    }
    fn hull(a: Iv, b: Iv) -> Iv {
        Iv::new(a.lo.min(b.lo), a.hi.max(b.hi))
    }
}

/// The tile: ranges of x and y, and the moment.
#[derive(Clone, Copy)]
pub struct Tile {
    pub x: Iv,
    pub y: Iv,
    pub t: f64,
}

impl Tile {
    fn key(&self) -> [u64; 4] {
        [self.x.lo.to_bits(), self.x.hi.to_bits(), self.y.lo.to_bits(), self.y.hi.to_bits()]
    }
}

type Key = (*const Field, u64, [u64; 4]);

pub struct Culler {
    bounds: HashMap<Key, Iv>,
    folded: HashMap<Key, Arc<Field>>,
}

impl Culler {
    pub fn new() -> Culler {
        Culler { bounds: HashMap::new(), folded: HashMap::new() }
    }

    pub fn bound(&mut self, f: &Field, tile: Tile, seed: u64) -> Iv {
        match f {
            Field::Const(c) => Iv::point(*c),
            Field::X => tile.x,
            Field::Y => tile.y,
            Field::T => Iv::point(tile.t),
            Field::Neg(a) => {
                let a = self.bound(a, tile, seed);
                Iv::new(-a.hi, -a.lo)
            }
            Field::Bin(op, a, b) => {
                let (a, b) = (self.bound(a, tile, seed), self.bound(b, tile, seed));
                bin(*op, a, b)
            }
            Field::Call(fun, args) => {
                let mut v = [ANY; 4];
                for (k, a) in args.iter().enumerate() {
                    v[k] = self.bound(a, tile, seed);
                }
                call(*fun, &v[..args.len()], seed)
            }
            Field::Grain { .. } => ANY,
            Field::Ref(r) | Field::Memo { inner: r, .. } => {
                let key = (Arc::as_ptr(r), seed, tile.key());
                if let Some(b) = self.bounds.get(&key) {
                    return *b;
                }
                let b = self.bound(r, tile, seed);
                self.bounds.insert(key, b);
                b
            }
            Field::Pattern(evs) => {
                if near(evs, tile).is_empty() { Iv::point(0.0) } else { ANY }
            }
            Field::Remap { inner, x, y } => {
                let (bx, by) = (self.bound(x, tile, seed), self.bound(y, tile, seed));
                self.bound(inner, Tile { x: bx, y: by, t: tile.t }, seed)
            }
            Field::Reseed { inner, seed: s } => self.bound(inner, tile, *s),
            Field::Raster { img, cx, cy, width, angle } => {
                let (cx, cy, w, a) = (
                    self.bound(cx, tile, seed).single(),
                    self.bound(cy, tile, seed).single(),
                    self.bound(width, tile, seed).single(),
                    self.bound(angle, tile, seed).single(),
                );
                match (cx, cy, w, a) {
                    (Some(cx), Some(cy), Some(w), Some(a)) if tile.x.finite() && tile.y.finite() => {
                        // The tile's centre in the picture's own frame, and the
                        // tile's half-diagonal: beyond the picture, exactly 0.
                        let h = w * img.aspect();
                        let (mx, my) = ((tile.x.lo + tile.x.hi) * 0.5 - cx, (tile.y.lo + tile.y.hi) * 0.5 - cy);
                        let r = 0.5 * ((tile.x.hi - tile.x.lo).hypot(tile.y.hi - tile.y.lo));
                        let (s, c) = (libm::sin(a), libm::cos(a));
                        let (u, v) = (mx * c + my * s, -mx * s + my * c);
                        let m = 1e-9 + 1e-6 * w.abs();
                        if u.abs() > w.abs() * 0.5 + r + m || v.abs() > h.abs() * 0.5 + r + m {
                            Iv::point(0.0)
                        } else {
                            UNIT
                        }
                    }
                    _ => UNIT,
                }
            }
            Field::Cache { .. } | Field::State(_) | Field::Grid { .. } | Field::Grow { .. } => ANY,
            Field::Curve(c) => Iv::point(crate::eval::curve_at(c, tile.t)),
            Field::Retime { inner, time } => match self.bound(time, tile, seed).single() {
                Some(c) => self.bound(inner, Tile { t: c, ..tile }, seed),
                None => ANY,
            },
        }
    }

    /// The field simplified for one tile.
    pub fn fold(&mut self, f: &Field, tile: Tile, seed: u64) -> Field {
        if let Some(v) = self.bound(f, tile, seed).single() {
            // Computing it once at any point of the tile gives that number,
            // with exactly the arithmetic of a full evaluation.
            let p = [tile.x.lo, tile.y.lo, tile.t];
            let e = f.eval(p, seed);
            if e == v {
                return Field::Const(v);
            }
        }
        match f {
            Field::Neg(a) => Field::Neg(Box::new(self.fold(a, tile, seed))),
            Field::Bin(op, a, b) => Field::Bin(*op, Box::new(self.fold(a, tile, seed)), Box::new(self.fold(b, tile, seed))),
            Field::Call(b, args) => Field::Call(*b, args.iter().map(|a| self.fold(a, tile, seed)).collect()),
            Field::Ref(r) => Field::Ref(self.fold_shared(r, tile, seed, false)),
            Field::Memo { inner, .. } => {
                let inner = self.fold_shared(inner, tile, seed, true);
                inner.as_ref().clone()
            }
            Field::Pattern(evs) => {
                let n = near(evs, tile);
                if n.len() == evs.len() { f.clone() } else { Field::Pattern(Arc::new(n)) }
            }
            Field::Remap { inner, x, y } => {
                let (bx, by) = (self.bound(x, tile, seed), self.bound(y, tile, seed));
                let inner = if bx.finite() && by.finite() {
                    Arc::new(self.fold(inner, Tile { x: bx, y: by, t: tile.t }, seed))
                } else {
                    inner.clone()
                };
                Field::Remap { inner, x: Box::new(self.fold(x, tile, seed)), y: Box::new(self.fold(y, tile, seed)) }
            }
            Field::Reseed { inner, seed: s } => {
                Field::Reseed { inner: Arc::new(self.fold(inner, tile, *s)), seed: *s }
            }
            Field::Retime { inner, time } => match self.bound(time, tile, seed).single() {
                Some(c) => self.fold(inner, Tile { t: c, ..tile }, seed),
                None => Field::Retime { inner: inner.clone(), time: Box::new(self.fold(time, tile, seed)) },
            },
            other => other.clone(),
        }
    }

    fn fold_shared(&mut self, r: &Arc<Field>, tile: Tile, seed: u64, memo: bool) -> Arc<Field> {
        let key = (Arc::as_ptr(r), seed, tile.key());
        if let Some(d) = self.folded.get(&key) {
            return d.clone();
        }
        let folded = self.fold(r, tile, seed);
        let d = if memo && !matches!(folded, Field::Const(_)) {
            Arc::new(Field::memo(folded))
        } else {
            Arc::new(folded)
        };
        self.folded.insert(key, d.clone());
        d
    }
}

/// The events whose spot can reach the tile (the others add exactly 0).
fn near(evs: &[Event], tile: Tile) -> Vec<Event> {
    if !tile.x.finite() || !tile.y.finite() {
        return evs.to_vec();
    }
    evs.iter()
        .filter(|e| {
            let dx = (tile.x.lo - e.x).max(0.0).max(e.x - tile.x.hi);
            let dy = (tile.y.lo - e.y).max(0.0).max(e.y - tile.y.hi);
            // A little inside the 4σ cut of Field::eval, to stay on the safe side.
            dx * dx + dy * dy <= 16.0 * e.size * e.size * (1.0 + 1e-9) + 1e-300
        })
        .cloned()
        .collect()
}

fn bin(op: BinOp, a: Iv, b: Iv) -> Iv {
    if let (Some(x), Some(y)) = (a.single(), b.single()) {
        return Iv::point(apply_bin(op, x, y));
    }
    let u = |lo: bool, hi: bool| Iv::new(if lo { 1.0 } else { 0.0 }, if hi { 1.0 } else { 0.0 });
    match op {
        BinOp::Add => Iv::new(a.lo + b.lo, a.hi + b.hi).widen(),
        BinOp::Sub => Iv::new(a.lo - b.hi, a.hi - b.lo).widen(),
        BinOp::Mul => {
            // 0 times anything finite is exactly 0.
            if (a.single() == Some(0.0) && b.finite()) || (b.single() == Some(0.0) && a.finite()) {
                return Iv::point(0.0);
            }
            let p = [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi];
            if p.iter().any(|v| v.is_nan()) {
                return ANY;
            }
            Iv::new(p.iter().cloned().fold(f64::INFINITY, f64::min), p.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
                .widen()
        }
        BinOp::Div => {
            if b.contains0() || !b.finite() {
                return ANY;
            }
            if a.single() == Some(0.0) {
                return Iv::point(0.0);
            }
            let p = [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi];
            if p.iter().any(|v| v.is_nan()) {
                return ANY;
            }
            Iv::new(p.iter().cloned().fold(f64::INFINITY, f64::min), p.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
                .widen()
        }
        BinOp::Mod => match b.single() {
            Some(m) if m > 0.0 => Iv::new(0.0, m),
            _ => ANY,
        },
        BinOp::Pow => match b.single() {
            Some(e) if e == 2.0 => {
                let (l, h) = (a.lo.abs(), a.hi.abs());
                let lo = if a.contains0() { 0.0 } else { l.min(h) };
                Iv::new(lo * lo, l.max(h) * l.max(h)).widen()
            }
            Some(e) if e > 0.0 && a.lo >= 0.0 => Iv::new(libm::pow(a.lo, e), libm::pow(a.hi, e)).widen(),
            _ => ANY,
        },
        BinOp::Lt => u(a.hi < b.lo, !(a.lo >= b.hi)),
        BinOp::Gt => u(a.lo > b.hi, !(a.hi <= b.lo)),
        BinOp::Le => u(a.hi <= b.lo, !(a.lo > b.hi)),
        BinOp::Ge => u(a.lo >= b.hi, !(a.hi < b.lo)),
        BinOp::Eq => u(false, !(a.hi < b.lo || b.hi < a.lo)),
        BinOp::Ne => u(a.hi < b.lo || b.hi < a.lo, true),
        BinOp::And => {
            let (az, bz) = (a.single() == Some(0.0), b.single() == Some(0.0));
            u(!a.contains0() && !b.contains0(), !(az || bz))
        }
        BinOp::Or => {
            let (az, bz) = (a.single() == Some(0.0), b.single() == Some(0.0));
            u(!a.contains0() || !b.contains0(), !(az && bz))
        }
    }
}

fn call(f: Builtin, v: &[Iv], seed: u64) -> Iv {
    if v.iter().all(|a| a.single().is_some()) {
        let p: Vec<f64> = v.iter().map(|a| a.lo).collect();
        return Iv::point(apply_builtin(f, &p, seed));
    }
    use Builtin::*;
    let a = v[0];
    match f {
        Sin | Cos => Iv::new(-1.0, 1.0),
        Tan => ANY,
        Abs => {
            if a.lo >= 0.0 {
                a
            } else if a.hi <= 0.0 {
                Iv::new(-a.hi, -a.lo)
            } else {
                Iv::new(0.0, (-a.lo).max(a.hi))
            }
        }
        Sqrt => {
            if a.lo >= 0.0 { Iv::new(a.lo.sqrt(), a.hi.sqrt()).widen() } else { ANY }
        }
        Exp => Iv::new(libm::exp(a.lo), libm::exp(a.hi)).widen(),
        Ln => {
            if a.lo > 0.0 { Iv::new(libm::log(a.lo), libm::log(a.hi)).widen() } else { ANY }
        }
        Floor => Iv::new(a.lo.floor(), a.hi.floor()),
        Fract => {
            if a.finite() && a.lo.floor() == a.hi.floor() {
                let k = a.lo.floor();
                Iv::new(a.lo - k, a.hi - k).widen()
            } else {
                Iv::new(0.0, 1.0)
            }
        }
        Sign => {
            if a.lo > 0.0 {
                Iv::point(1.0)
            } else if a.hi < 0.0 {
                Iv::point(-1.0)
            } else {
                Iv::new(-1.0, 1.0)
            }
        }
        Min => Iv::new(a.lo.min(v[1].lo), a.hi.min(v[1].hi)),
        Max => Iv::new(a.lo.max(v[1].lo), a.hi.max(v[1].hi)),
        Atan2 => Iv::new(-PI, PI),
        Step => {
            let e = a;
            let x = v[1];
            if x.lo >= e.hi {
                Iv::point(1.0)
            } else if x.hi < e.lo {
                Iv::point(0.0)
            } else {
                UNIT
            }
        }
        Clamp => match (v[1].single(), v[2].single()) {
            (Some(l), Some(h)) => {
                let (l, h) = (l.min(h), l.max(h));
                Iv::new(a.lo.clamp(l, h), a.hi.clamp(l, h))
            }
            _ => Iv::hull(v[1], v[2]),
        },
        Mix => {
            let (x, y, t) = (v[0], v[1], v[2]);
            bin(BinOp::Add, x, bin(BinOp::Mul, bin(BinOp::Sub, y, x), t))
        }
        Smoothstep => match (v[0].single(), v[1].single()) {
            (Some(e0), Some(e1)) if e0 != e1 => {
                let s = |x: f64| apply_builtin(Smoothstep, &[e0, e1, x], seed);
                let x = v[2];
                if !x.finite() {
                    return UNIT;
                }
                let (p, q) = (s(x.lo), s(x.hi));
                // Monotone in x, rising or falling: its ends bound it.
                Iv::new(p.min(q), p.max(q)).widen()
            }
            _ => UNIT,
        },
        Distance => {
            let dx = bin(BinOp::Sub, v[0], v[2]);
            let dy = bin(BinOp::Sub, v[1], v[3]);
            if !dx.finite() || !dy.finite() {
                return ANY;
            }
            let near = |d: Iv| if d.contains0() { 0.0 } else { d.lo.abs().min(d.hi.abs()) };
            let far = |d: Iv| d.lo.abs().max(d.hi.abs());
            Iv::new(near(dx).hypot(near(dy)), far(dx).hypot(far(dy))).widen()
        }
        Noise => Iv::new(-1.0, 1.0),
    }
}
