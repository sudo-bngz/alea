//! Resolution: from a work (a space of possibilities) to an edition (one draw).
//! After this step no undrawn distribution remains — except `~~` grains,
//! which are deterministic functions of the point and the seed.

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::ast::*;
use crate::error::{Error, Res, Span};
use crate::library::Module;

pub const MAX_ATTEMPTS: u32 = 1000;
const MAX_DEPTH: usize = 16;
pub const RENDER_KINDS: &[&str] = &["image", "sound", "video", "sculpture"];
const EVENT_ATTRS: &[&str] = &["x", "y", "t", "intensity", "size"];

// ───────────────────────────── distributions ─────────────────────────────

#[derive(Clone, Debug)]
pub enum Dist {
    Uniform(f64, f64),
    Normal(f64, f64),
    Beta(f64, f64),
    Poisson(f64),
    Exponential(f64),
}

// The samplers are written here on top of `libm` (pure Rust maths) rather
// than taken from a library that calls the platform's maths: `sin`, `exp` or
// `ln` can differ in their last bit between operating systems and WebAssembly,
// and a draw must give the same value everywhere for a seed to replay the same
// edition in the browser and on the command line.

/// Uniform in (0, 1], never 0, so it can go through `ln`.
fn open01<R: Rng + ?Sized>(r: &mut R) -> f64 {
    1.0 - r.random::<f64>()
}

/// Standard normal (Box–Muller).
fn std_normal<R: Rng + ?Sized>(r: &mut R) -> f64 {
    let (u, v) = (open01(r), r.random::<f64>());
    libm::sqrt(-2.0 * libm::log(u)) * libm::cos(TAU * v)
}

/// Gamma(k, 1), Marsaglia & Tsang (2000).
fn gamma<R: Rng + ?Sized>(k: f64, r: &mut R) -> f64 {
    if k < 1.0 {
        return gamma(k + 1.0, r) * libm::pow(open01(r), 1.0 / k);
    }
    let d = k - 1.0 / 3.0;
    let c = 1.0 / libm::sqrt(9.0 * d);
    loop {
        let x = std_normal(r);
        let v = 1.0 + c * x;
        if v <= 0.0 {
            continue;
        }
        let v = v * v * v;
        let u = open01(r);
        if libm::log(u) < 0.5 * x * x + d - d * v + d * libm::log(v) {
            return d * v;
        }
    }
}

/// Poisson: multiplication for small λ, transformed rejection (Hörmann's PTRS) otherwise.
fn poisson<R: Rng + ?Sized>(lambda: f64, r: &mut R) -> f64 {
    if lambda < 30.0 {
        let limit = libm::exp(-lambda);
        let (mut k, mut p) = (0.0, r.random::<f64>());
        while p > limit {
            k += 1.0;
            p *= r.random::<f64>();
        }
        return k;
    }
    let slam = libm::sqrt(lambda);
    let loglam = libm::log(lambda);
    let b = 0.931 + 2.53 * slam;
    let a = -0.059 + 0.02483 * b;
    let inv_alpha = 1.1239 + 1.1328 / (b - 3.4);
    let vr = 0.9277 - 3.6224 / (b - 2.0);
    loop {
        let u = r.random::<f64>() - 0.5;
        let v = open01(r);
        let us = 0.5 - u.abs();
        let k = libm::floor((2.0 * a / us + b) * u + lambda + 0.43);
        if us >= 0.07 && v <= vr {
            return k;
        }
        if k < 0.0 || (us < 0.013 && v > us) {
            continue;
        }
        let lhs = libm::log(v) + libm::log(inv_alpha) - libm::log(a / (us * us) + b);
        if lhs <= -lambda + k * loglam - libm::lgamma(k + 1.0) {
            return k;
        }
    }
}

impl Dist {
    /// Parameters are validated at construction.
    pub fn draw<R: Rng + ?Sized>(&self, r: &mut R) -> f64 {
        match *self {
            Dist::Uniform(a, b) => a + (b - a) * r.random::<f64>(),
            Dist::Normal(m, s) => m + s * std_normal(r),
            Dist::Beta(a, b) => {
                let (x, y) = (gamma(a, r), gamma(b, r));
                x / (x + y)
            }
            Dist::Poisson(l) => poisson(l, r),
            Dist::Exponential(l) => -libm::log(open01(r)) / l,
        }
    }
}

// ───────────────────────────── fields ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Builtin {
    Sin,
    Cos,
    Tan,
    Abs,
    Sqrt,
    Exp,
    Ln,
    Floor,
    Fract,
    Sign,
    Min,
    Max,
    Atan2,
    Step,
    Clamp,
    Mix,
    Smoothstep,
    Distance,
    Noise,
}

impl Builtin {
    fn lookup(name: &str) -> Option<(Builtin, usize)> {
        use Builtin::*;
        Some(match name {
            "sin" => (Sin, 1),
            "cos" => (Cos, 1),
            "tan" => (Tan, 1),
            "abs" => (Abs, 1),
            "sqrt" => (Sqrt, 1),
            "exp" => (Exp, 1),
            "ln" => (Ln, 1),
            "floor" => (Floor, 1),
            "fract" => (Fract, 1),
            "sign" => (Sign, 1),
            "min" => (Min, 2),
            "max" => (Max, 2),
            "atan2" => (Atan2, 2),
            "step" => (Step, 2),
            "clamp" => (Clamp, 3),
            "mix" => (Mix, 3),
            "smoothstep" => (Smoothstep, 3),
            "distance" => (Distance, 4),
            "noise" => (Noise, 3),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub x: f64,
    pub y: f64,
    pub t: f64,
    pub intensity: f64,
    pub size: f64,
}

/// A resolved field: a function of (x, y, t) whose `~` draws have all been made.
#[derive(Clone, Debug)]
pub enum Field {
    Const(f64),
    X,
    Y,
    T,
    Neg(Box<Field>),
    Bin(BinOp, Box<Field>, Box<Field>),
    Call(Builtin, Vec<Field>),
    /// `~~d`: a different draw at every point, yet reproducible (hashed).
    Grain { dist: Dist, id: u64 },
    Ref(Arc<Field>),
    /// A pattern seen as a field: a "long exposure" of its events.
    Pattern(Arc<Vec<Event>>),
    /// Evaluates `inner` at moved coordinates (x', y'), themselves fields of
    /// the original point. Every transform (move, rotate, tile, warp…) is one.
    Remap { inner: Arc<Field>, x: Box<Field>, y: Box<Field> },
    /// Evaluates `inner` with its own seed: each instance of a work brought in
    /// with `use` keeps its own grain and noise, distinct from its siblings'.
    Reseed { inner: Arc<Field>, seed: u64 },
    /// A bitmap (text or picture) of width `width`, centred on (cx, cy) and
    /// turned by `angle` radians; its height follows the bitmap's proportions.
    Raster { img: Arc<crate::raster::Raster>, cx: Box<Field>, cy: Box<Field>, width: Box<Field>, angle: Box<Field> },
    /// `inner`, already computed at every pixel of one frame grid (see
    /// [`Field::cached_for_frames`]). Off that grid, `inner` is evaluated.
    Cache { inner: Arc<Field>, grid: Arc<FrameCache> },
    /// A shared part, remembered at the last point it was evaluated: a name
    /// used three times at the same pixel is computed once. `id` is unique.
    Memo { inner: Arc<Field>, id: u64 },
    /// Inside a `grow` rule: the previous generation of grow field `k`.
    State(usize),
    /// One generation of a grid, read cell by cell (no smoothing).
    Grid { data: Arc<Vec<f64>>, res: usize, di: i64, dj: i64 },
    /// A grown field: its generations, smoothly interpolated in space and time.
    Grow { data: Arc<GrowData>, k: usize },
    /// `at(field, time)`: `inner` seen at another moment, `time` (itself a field).
    Retime { inner: Arc<Field>, time: Box<Field> },
}

/// The recorded generations of a `grow` block.
#[derive(Debug)]
pub struct GrowData {
    pub res: usize,
    /// How many generations were kept, evenly spaced from the first to the last.
    pub snaps: usize,
    /// For each field: `snaps` grids of res × res values, one after the other.
    pub fields: Vec<Vec<f32>>,
}

impl GrowData {
    /// Bilinear in space (the grid wraps around, like a torus), linear in time.
    fn sample(&self, k: usize, p: [f64; 3]) -> f64 {
        let n = self.res;
        let s = p[2].clamp(0.0, 1.0) * (self.snaps - 1) as f64;
        let j0 = (s.floor() as usize).min(self.snaps - 1);
        let j1 = (j0 + 1).min(self.snaps - 1);
        let ft = s - j0 as f64;
        let (u, v) = (p[0] * n as f64 - 0.5, p[1] * n as f64 - 0.5);
        if !u.is_finite() || !v.is_finite() {
            return 0.0;
        }
        let (fu, fv) = (u.floor(), v.floor());
        let (ax, ay) = (u - fu, v - fv);
        let wrap = |a: f64| a.rem_euclid(n as f64) as usize;
        let (i0, i1, r0, r1) = (wrap(fu), wrap(fu + 1.0), wrap(fv), wrap(fv + 1.0));
        let g = &self.fields[k];
        let at = |j: usize| {
            let b = j * n * n;
            let c = |r: usize, i: usize| g[b + r * n + i] as f64;
            let top = c(r0, i0) + (c(r0, i1) - c(r0, i0)) * ax;
            let bot = c(r1, i0) + (c(r1, i1) - c(r1, i0)) * ax;
            top + (bot - top) * ay
        };
        let a = at(j0);
        if ft == 0.0 { a } else { a + (at(j1) - a) * ft }
    }
}

const MEMO_SLOTS: usize = 4096;
static MEMO_IDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

thread_local! {
    static MEMO: std::cell::RefCell<Vec<(u64, [f64; 3], u64, f64)>> =
        std::cell::RefCell::new(vec![(0, [0.0; 3], 0, 0.0); MEMO_SLOTS]);
}

/// The values of a field that doesn't change with time, at every pixel of a
/// `w`×`h` frame laid out as in [`crate::media::sample_frame`].
#[derive(Debug)]
pub struct FrameCache {
    pub w: usize,
    pub h: usize,
    pub vals: Vec<f64>,
}

impl FrameCache {
    fn lookup(&self, p: [f64; 3]) -> Option<f64> {
        let s = self.w.min(self.h) as f64;
        let (ox, oy) = (0.5 - self.w as f64 / (2.0 * s), 0.5 - self.h as f64 / (2.0 * s));
        let (fx, fy) = (((p[0] - ox) * s - 0.5).round(), ((p[1] - oy) * s - 0.5).round());
        if fx < 0.0 || fy < 0.0 || fx >= self.w as f64 || fy >= self.h as f64 {
            return None;
        }
        // Only exactly on the grid: the same arithmetic as sample_frame.
        if ox + (fx + 0.5) / s != p[0] || oy + (fy + 0.5) / s != p[1] {
            return None;
        }
        Some(self.vals[fy as usize * self.w + fx as usize])
    }
}

/// What a field reads: the position (x or y) and the time.
#[derive(Clone, Copy, Default)]
struct Reads {
    xy: bool,
    t: bool,
}

impl std::ops::BitOr for Reads {
    type Output = Reads;
    fn bitor(self, o: Reads) -> Reads {
        Reads { xy: self.xy || o.xy, t: self.t || o.t }
    }
}

type Memo<T> = HashMap<(*const Field, u64), T>;

impl Field {
    fn reads(&self, memo: &mut HashMap<*const Field, Reads>) -> Reads {
        let xy = Reads { xy: true, t: false };
        match self {
            Field::Const(_) => Reads::default(),
            Field::X | Field::Y => xy,
            Field::T => Reads { xy: false, t: true },
            Field::Neg(a) => a.reads(memo),
            Field::Bin(_, a, b) => a.reads(memo) | b.reads(memo),
            Field::Call(_, args) => args.iter().fold(Reads::default(), |r, a| r | a.reads(memo)),
            Field::Grain { .. } => Reads { xy: true, t: true },
            Field::Ref(f) => {
                let key = Arc::as_ptr(f);
                if let Some(r) = memo.get(&key) {
                    return *r;
                }
                let r = f.reads(memo);
                memo.insert(key, r);
                r
            }
            Field::Pattern(_) => xy,
            Field::Remap { inner, x, y } => {
                let (i, c) = (inner.reads(memo), x.reads(memo) | y.reads(memo));
                // `inner` sees moved coordinates: it reads x, y (and their
                // time) only through them.
                if i.xy { Reads { xy: c.xy, t: i.t || c.t } } else { i }
            }
            Field::Reseed { inner, .. } => inner.reads(memo),
            Field::Raster { cx, cy, width, angle, .. } => {
                xy | cx.reads(memo) | cy.reads(memo) | width.reads(memo) | angle.reads(memo)
            }
            Field::Cache { inner, .. } => Reads { xy: true, t: inner.reads(memo).t },
            Field::Memo { inner, .. } => inner.reads(memo),
            Field::State(_) | Field::Grid { .. } => xy,
            Field::Grow { .. } => Reads { xy: true, t: true },
            Field::Retime { inner, time } => {
                let (i, tm) = (inner.reads(memo), time.reads(memo));
                // the inner field sees the time only through `time`
                if i.t { Reads { xy: i.xy || tm.xy, t: tm.t } } else { i }
            }
        }
    }

    /// Wraps a shared part so that it is computed once per point.
    pub fn memo(inner: Field) -> Field {
        let id = MEMO_IDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Field::Memo { inner: Arc::new(inner), id }
    }

    /// Debugging: a census of the distinct nodes, by kind.
    pub fn census(&self) -> String {
        fn walk(f: &Field, seen: &mut std::collections::HashSet<*const Field>, n: &mut HashMap<&'static str, usize>) {
            let k = match f {
                Field::Const(_) => "const", Field::X | Field::Y | Field::T => "var", Field::Neg(_) => "neg",
                Field::Bin(..) => "bin", Field::Call(..) => "call", Field::Grain { .. } => "grain",
                Field::Ref(_) => "ref", Field::Pattern(_) => "pattern", Field::Remap { .. } => "remap",
                Field::Reseed { .. } => "reseed", Field::Raster { .. } => "raster", Field::Cache { .. } => "cache",
                Field::Memo { .. } => "memo",
                Field::State(_) => "state", Field::Grid { .. } => "grid", Field::Grow { .. } => "grow",
                Field::Retime { .. } => "retime",
            };
            *n.entry(k).or_default() += 1;
            match f {
                Field::Neg(a) => walk(a, seen, n),
                Field::Bin(_, a, b) => { walk(a, seen, n); walk(b, seen, n) }
                Field::Call(_, a) => a.iter().for_each(|a| walk(a, seen, n)),
                Field::Ref(r) | Field::Memo { inner: r, .. } | Field::Reseed { inner: r, .. } => {
                    if seen.insert(Arc::as_ptr(r)) { walk(r, seen, n) }
                }
                Field::Remap { inner, x, y } => { walk(inner, seen, n); walk(x, seen, n); walk(y, seen, n) }
                Field::Raster { cx, cy, width, angle, .. } => { walk(cx, seen, n); walk(cy, seen, n); walk(width, seen, n); walk(angle, seen, n) }
                _ => {}
            }
        }
        let mut n = HashMap::new();
        walk(self, &mut std::collections::HashSet::new(), &mut n);
        let mut v: Vec<_> = n.into_iter().collect();
        v.sort();
        format!("{v:?} cost={}", self.cost(&mut HashMap::new()))
    }

    /// Roughly how much work one evaluation takes (capped).
    fn cost(&self, memo: &mut HashMap<*const Field, usize>) -> usize {
        const CAP: usize = 4096;
        let c = match self {
            Field::Neg(a) => 1 + a.cost(memo),
            Field::Bin(_, a, b) => 1 + a.cost(memo) + b.cost(memo),
            Field::Call(_, args) => 1 + args.iter().map(|a| a.cost(memo)).sum::<usize>(),
            Field::Ref(f) | Field::Memo { inner: f, .. } => {
                let key = Arc::as_ptr(f);
                if let Some(c) = memo.get(&key) {
                    return *c;
                }
                let c = f.cost(memo);
                memo.insert(key, c);
                c
            }
            Field::Remap { inner, x, y } => 1 + inner.cost(memo) + x.cost(memo) + y.cost(memo),
            Field::Reseed { inner, .. } => inner.cost(memo),
            Field::Pattern(e) => 4 * e.len(),
            Field::Raster { .. } => 24,
            Field::Retime { inner, time } => 1 + inner.cost(memo) + time.cost(memo),
            _ => 1,
        };
        c.min(CAP)
    }

    /// The same field at a fixed moment `t`: every part that doesn't depend
    /// on the position (a moving circle's centre, a fade) becomes a number,
    /// computed once instead of at every pixel. Shared parts stay shared.
    pub fn at_time(&self, t: f64, seed: u64) -> Field {
        let mut reads = HashMap::new();
        let mut done = Memo::new();
        self.fold_t(t, seed, &mut reads, &mut done)
    }

    fn fold_t(&self, t: f64, seed: u64, reads: &mut HashMap<*const Field, Reads>, done: &mut Memo<Arc<Field>>) -> Field {
        if !self.reads(reads).xy {
            return Field::Const(self.eval([0.5, 0.5, t], seed));
        }
        let mut f = |a: &Field| Box::new(a.fold_t(t, seed, reads, done));
        match self {
            Field::Neg(a) => Field::Neg(f(a)),
            Field::Bin(op, a, b) => Field::Bin(*op, f(a), f(b)),
            Field::Call(b, args) => Field::Call(*b, args.iter().map(|a| *f(a)).collect()),
            Field::Ref(r) => {
                let key = (Arc::as_ptr(r), seed);
                if let Some(d) = done.get(&key) {
                    return Field::Ref(d.clone());
                }
                let folded = r.fold_t(t, seed, reads, done);
                let d = if folded.cost(&mut HashMap::new()) >= 24 {
                    Arc::new(Field::memo(folded))
                } else {
                    Arc::new(folded)
                };
                done.insert(key, d.clone());
                Field::Ref(d)
            }
            Field::Remap { inner, x, y } => {
                Field::Remap { inner: Arc::new(*f(inner)), x: f(x), y: f(y) }
            }
            Field::Reseed { inner, seed: s } => {
                Field::Reseed { inner: Arc::new(inner.fold_t(t, *s, reads, done)), seed: *s }
            }
            Field::Raster { img, cx, cy, width, angle } => {
                Field::Raster { img: img.clone(), cx: f(cx), cy: f(cy), width: f(width), angle: f(angle) }
            }
            Field::Retime { inner, time } => {
                let time = f(time);
                match time.constant() {
                    // one moment for the whole frame: the inner field at that moment
                    Some(c) => inner.at_time(c, seed),
                    None => Field::Retime { inner: inner.clone(), time },
                }
            }
            other => other.clone(),
        }
    }

    /// Prepares a field for many frames of the same `w`×`h` size: the largest
    /// parts that don't change with time, and that are read at the pixel
    /// itself (not through a transform), are computed once for the whole
    /// animation. At most `budget` such parts are kept.
    pub fn cached_for_frames(&self, w: usize, h: usize, seed: u64, budget: usize) -> Field {
        let mut reads = HashMap::new();
        let mut done = Memo::new();
        let mut left = budget;
        self.cache_static(w, h, seed, &mut reads, &mut done, &mut left)
    }

    fn cache_static(
        &self,
        w: usize,
        h: usize,
        seed: u64,
        reads: &mut HashMap<*const Field, Reads>,
        done: &mut Memo<Arc<Field>>,
        left: &mut usize,
    ) -> Field {
        let r = self.reads(reads);
        let trivial = matches!(self, Field::Const(_) | Field::X | Field::Y | Field::T | Field::Cache { .. });
        // Only parts worth a whole frame of memory: a few nodes are cheaper to recompute.
        if r.xy && !r.t && !trivial && *left > 0 && self.cost(&mut HashMap::new()) >= 64 {
            *left -= 1;
            let vals = crate::media::sample_frame(self, w, h, 0.0, seed);
            return Field::Cache { inner: Arc::new(self.clone()), grid: Arc::new(FrameCache { w, h, vals }) };
        }
        if !r.xy || !r.t {
            return self.clone();
        }
        let mut f = |a: &Field| Box::new(a.cache_static(w, h, seed, reads, done, left));
        match self {
            Field::Neg(a) => Field::Neg(f(a)),
            Field::Bin(op, a, b) => Field::Bin(*op, f(a), f(b)),
            Field::Call(b, args) => Field::Call(*b, args.iter().map(|a| *f(a)).collect()),
            Field::Ref(rf) => {
                let key = (Arc::as_ptr(rf), seed);
                if let Some(d) = done.get(&key) {
                    return Field::Ref(d.clone());
                }
                let d = Arc::new(rf.cache_static(w, h, seed, reads, done, left));
                done.insert(key, d.clone());
                Field::Ref(d)
            }
            // A transform's coordinates are read at the pixel; what it moves is not.
            Field::Remap { inner, x, y } => Field::Remap { inner: inner.clone(), x: f(x), y: f(y) },
            Field::Reseed { inner, seed: s } => {
                Field::Reseed { inner: Arc::new(inner.cache_static(w, h, *s, reads, done, left)), seed: *s }
            }
            other => other.clone(),
        }
    }
}

impl Field {
    pub fn eval(&self, p: [f64; 3], seed: u64) -> f64 {
        match self {
            Field::Const(c) => *c,
            Field::X => p[0],
            Field::Y => p[1],
            Field::T => p[2],
            Field::Neg(a) => -a.eval(p, seed),
            Field::Bin(op, a, b) => apply_bin(*op, a.eval(p, seed), b.eval(p, seed)),
            Field::Call(f, args) => {
                let mut v = [0.0; 4];
                for (k, a) in args.iter().enumerate() {
                    v[k] = a.eval(p, seed);
                }
                apply_builtin(*f, &v[..args.len()], seed)
            }
            Field::Grain { dist, id } => {
                let h = hash(&[seed, *id, p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]);
                dist.draw(&mut SplitMix(h))
            }
            Field::Ref(f) => f.eval(p, seed),
            Field::Remap { inner, x, y } => inner.eval([x.eval(p, seed), y.eval(p, seed), p[2]], seed),
            Field::Reseed { inner, seed } => inner.eval(p, *seed),
            Field::Raster { img, cx, cy, width, angle } => {
                let w = width.eval(p, seed);
                let h = w * img.aspect();
                let (dx, dy) = (p[0] - cx.eval(p, seed), p[1] - cy.eval(p, seed));
                let a = angle.eval(p, seed);
                let (s, c) = (libm::sin(a), libm::cos(a));
                let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
                img.sample(u / w + 0.5, v / h + 0.5)
            }
            Field::Cache { inner, grid } => grid.lookup(p).unwrap_or_else(|| inner.eval(p, seed)),
            Field::State(_) => 0.0,
            Field::Grid { data, res, di, dj } => {
                let n = *res as i64;
                let cell = |v: f64| {
                    let t = v as i64; // a floor without the library call
                    if (t as f64) > v { t - 1 } else { t }
                };
                let i = (cell(p[0] * n as f64) + di).rem_euclid(n) as usize;
                let j = (cell(p[1] * n as f64) + dj).rem_euclid(n) as usize;
                data[j * *res + i]
            }
            Field::Grow { data, k } => data.sample(*k, p),
            Field::Retime { inner, time } => inner.eval([p[0], p[1], time.eval(p, seed)], seed),
            Field::Memo { inner, id } => {
                let k = (*id as usize) % MEMO_SLOTS;
                let hit = MEMO.with(|m| {
                    let e = m.borrow()[k];
                    (e.0 == *id && e.1 == p && e.2 == seed).then_some(e.3)
                });
                hit.unwrap_or_else(|| {
                    let v = inner.eval(p, seed);
                    MEMO.with(|m| m.borrow_mut()[k] = (*id, p, seed, v));
                    v
                })
            }
            Field::Pattern(evs) => evs
                .iter()
                .map(|e| {
                    let (dx, dy) = (p[0] - e.x, p[1] - e.y);
                    let (d2, s2) = (dx * dx + dy * dy, e.size * e.size);
                    // beyond 4σ the contribution is < 0.04 %: skip it
                    if d2 > 16.0 * s2 { 0.0 } else { e.intensity * libm::exp(-d2 / (2.0 * s2)) }
                })
                .sum(),
        }
    }

    pub fn constant(&self) -> Option<f64> {
        match self {
            Field::Const(c) => Some(*c),
            Field::Ref(f) => f.constant(),
            Field::Memo { inner, .. } => inner.constant(),
            Field::Remap { inner, .. } => inner.constant(),
            Field::Reseed { inner, .. } => inner.constant(),
            _ => None,
        }
    }

    pub fn pattern(&self) -> Option<&Arc<Vec<Event>>> {
        match self {
            Field::Pattern(p) => Some(p),
            Field::Ref(f) => f.pattern(),
            Field::Reseed { inner, .. } => inner.pattern(),
            _ => None,
        }
    }
}

pub fn apply_bin_pub(op: BinOp, a: f64, b: f64) -> f64 {
    apply_bin(op, a, b)
}

pub fn apply_builtin_pub(f: Builtin, v: &[f64], seed: u64) -> f64 {
    apply_builtin(f, v, seed)
}

fn apply_bin(op: BinOp, a: f64, b: f64) -> f64 {
    let truth = |c: bool| if c { 1.0 } else { 0.0 };
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Mod => a.rem_euclid(b),
        BinOp::Pow => libm::pow(a, b),
        BinOp::Lt => truth(a < b),
        BinOp::Gt => truth(a > b),
        BinOp::Le => truth(a <= b),
        BinOp::Ge => truth(a >= b),
        BinOp::Eq => truth(a == b),
        BinOp::Ne => truth(a != b),
        BinOp::And => truth(a != 0.0 && b != 0.0),
        BinOp::Or => truth(a != 0.0 || b != 0.0),
    }
}

fn apply_builtin(f: Builtin, v: &[f64], seed: u64) -> f64 {
    use Builtin::*;
    match f {
        Sin => libm::sin(v[0]),
        Cos => libm::cos(v[0]),
        Tan => libm::tan(v[0]),
        Abs => v[0].abs(),
        Sqrt => v[0].sqrt(),
        Exp => libm::exp(v[0]),
        Ln => libm::log(v[0]),
        Floor => v[0].floor(),
        Fract => v[0] - v[0].floor(),
        Sign => v[0].signum(),
        Min => v[0].min(v[1]),
        Max => v[0].max(v[1]),
        Atan2 => libm::atan2(v[0], v[1]),
        Step => if v[1] >= v[0] { 1.0 } else { 0.0 },
        Clamp => v[0].clamp(v[1].min(v[2]), v[1].max(v[2])),
        Mix => v[0] + (v[1] - v[0]) * v[2],
        Smoothstep => {
            let t = ((v[2] - v[0]) / (v[1] - v[0])).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        }
        Distance => ((v[0] - v[2]).powi(2) + (v[1] - v[3]).powi(2)).sqrt(),
        Noise => value_noise(v[0], v[1], v[2], seed),
    }
}

/// A tiny generator for grains, identical on every platform (unlike `SmallRng`,
/// which is a different algorithm on 32-bit targets such as WebAssembly).
struct SplitMix(u64);

impl RngCore for SplitMix {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        splitmix(self.0)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for chunk in dst.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The value of a `~~` grain at a point: the same draw as `Field::eval` makes.
pub fn grain_value(dist: &Dist, id: u64, p: [f64; 3], seed: u64) -> f64 {
    let h = hash(&[seed, id, p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]);
    dist.draw(&mut SplitMix(h))
}

pub fn hash(vals: &[u64]) -> u64 {
    vals.iter().fold(0xA1EA, |h, v| splitmix(h ^ v))
}

/// Smooth 3D value noise in [-1, 1], unique to each edition.
fn value_noise(x: f64, y: f64, z: f64, seed: u64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let (sx, sy, sz) = (smooth(x - xi), smooth(y - yi), smooth(z - zi));
    let corner = |dx: f64, dy: f64, dz: f64| {
        let h = hash(&[
            seed ^ 0xB0B,
            (xi + dx) as i64 as u64,
            (yi + dy) as i64 as u64,
            (zi + dz) as i64 as u64,
        ]);
        (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let plane = |dz: f64| {
        lerp(
            lerp(corner(0., 0., dz), corner(1., 0., dz), sx),
            lerp(corner(0., 1., dz), corner(1., 1., dz), sx),
            sy,
        )
    };
    lerp(plane(0.0), plane(1.0), sz)
}

/// Runs a `grow` block: generation 0 from `starts`, then each generation from
/// the one before through `nexts`, keeping evenly spaced generations.
fn simulate(starts: &[Field], nexts: &[Field], res: usize, steps: usize, seed: u64) -> GrowData {
    let n = res;
    let cells = n * n;
    let count = starts.len();
    // At most ~64 million values kept (f32), and never more than 600 moments.
    let snaps = (steps + 1).min((64_000_000 / (cells * count)).clamp(2, 600));
    let target = |j: usize| ((j as f64) * steps as f64 / (snaps - 1) as f64).round() as usize;
    let grid = |f: &Field, t: f64| -> Vec<f64> {
        // compiled once per generation, then run at every cell
        let prog = crate::compile::Program::new(&f.at_time(t, seed), seed);
        crate::media::par_map(n, |j| {
            let y = (j as f64 + 0.5) / n as f64;
            let mut slots = prog.slots(t);
            (0..n)
                .map(|i| {
                    let v = prog.run(&mut slots, (i as f64 + 0.5) / n as f64, y);
                    if v.is_finite() { v } else { 0.0 }
                })
                .collect::<Vec<f64>>()
        })
        .concat()
    };
    // What a rule reads that never changes (a map of the chemistry, a mask)
    // is computed once for the whole run, not at every generation.
    let nexts: Vec<Field> = {
        let mut memo = HashMap::new();
        nexts.iter().map(|f| precompute_static(f, n, seed, &grid, &mut memo)).collect()
    };
    let mut fields: Vec<Vec<f32>> = vec![Vec::with_capacity(snaps * cells); count];
    let mut cur: Vec<Arc<Vec<f64>>> = starts.iter().map(|f| Arc::new(grid(f, 0.0))).collect();
    let keep = |fields: &mut Vec<Vec<f32>>, cur: &[Arc<Vec<f64>>]| {
        for (k, g) in cur.iter().enumerate() {
            fields[k].extend(g.iter().map(|v| *v as f32));
        }
    };
    keep(&mut fields, &cur);
    let mut j = 1;
    for g in 0..steps {
        let t = g as f64 / steps as f64;
        let next: Vec<Arc<Vec<f64>>> = nexts
            .iter()
            .map(|f| Arc::new(grid(&bind_state(f, &cur, n, &mut HashMap::new()), t)))
            .collect();
        cur = next;
        while j < snaps && target(j) == g + 1 {
            keep(&mut fields, &cur);
            j += 1;
        }
    }
    GrowData { res: n, snaps, fields }
}

/// Does a field read the previous generation (`State`)?
fn has_state(f: &Field, memo: &mut HashMap<*const Field, bool>) -> bool {
    match f {
        Field::State(_) => true,
        Field::Neg(a) => has_state(a, memo),
        Field::Bin(_, a, b) => has_state(a, memo) || has_state(b, memo),
        Field::Call(_, args) => args.iter().any(|a| has_state(a, memo)),
        Field::Ref(r) | Field::Memo { inner: r, .. } | Field::Reseed { inner: r, .. } => {
            let key = Arc::as_ptr(r);
            if let Some(v) = memo.get(&key) {
                return *v;
            }
            let v = has_state(r, memo);
            memo.insert(key, v);
            v
        }
        Field::Remap { inner, x, y } => has_state(inner, memo) || has_state(x, memo) || has_state(y, memo),
        Field::Raster { cx, cy, width, angle, .. } => [cx, cy, width, angle].iter().any(|a| has_state(a, memo)),
        Field::Retime { inner, time } => has_state(inner, memo) || has_state(time, memo),
        _ => false,
    }
}

/// Replaces the parts of a grow rule that depend neither on the previous
/// generation nor on time by their values on the grid, computed once.
fn precompute_static(
    f: &Field,
    n: usize,
    seed: u64,
    grid: &dyn Fn(&Field, f64) -> Vec<f64>,
    memo: &mut HashMap<*const Field, Arc<Field>>,
) -> Field {
    let r = f.reads(&mut HashMap::new());
    let trivial = matches!(f, Field::Const(_) | Field::X | Field::Y | Field::T | Field::Grid { .. } | Field::State(_));
    if r.xy && !r.t && !trivial && !has_state(f, &mut HashMap::new()) && f.cost(&mut HashMap::new()) >= 6 && std::env::var("ALEA_NOPRE").is_err() {
        return Field::Grid { data: Arc::new(grid(f, 0.0)), res: n, di: 0, dj: 0 };
    }
    let mut p = |a: &Field| Box::new(precompute_static(a, n, seed, grid, memo));
    match f {
        Field::Neg(a) => Field::Neg(p(a)),
        Field::Bin(op, a, b) => Field::Bin(*op, p(a), p(b)),
        Field::Call(c, args) => Field::Call(*c, args.iter().map(|a| *p(a)).collect()),
        Field::Ref(rf) => {
            let key = Arc::as_ptr(rf);
            if let Some(d) = memo.get(&key) {
                return Field::Ref(d.clone());
            }
            let d = Arc::new(precompute_static(rf, n, seed, grid, memo));
            memo.insert(key, d.clone());
            Field::Ref(d)
        }
        // Inside a transform the points are moved: leave it as it is.
        other => other.clone(),
    }
}

/// The previous generation put in place of `State(k)`.
fn bind_state(f: &Field, cur: &[Arc<Vec<f64>>], res: usize, memo: &mut HashMap<*const Field, Arc<Field>>) -> Field {
    let mut b = |a: &Field| Box::new(bind_state(a, cur, res, memo));
    match f {
        Field::State(k) => Field::Grid { data: cur[*k].clone(), res, di: 0, dj: 0 },
        Field::Neg(a) => Field::Neg(b(a)),
        Field::Bin(op, x, y) => Field::Bin(*op, b(x), b(y)),
        Field::Call(c, args) => Field::Call(*c, args.iter().map(|a| *b(a)).collect()),
        Field::Ref(r) => {
            let key = Arc::as_ptr(r);
            if let Some(d) = memo.get(&key) {
                return Field::Ref(d.clone());
            }
            let d = Arc::new(bind_state(r, cur, res, memo));
            memo.insert(key, d.clone());
            Field::Ref(d)
        }
        Field::Remap { inner, x, y } => {
            // A neighbour of the previous generation: read its cell directly.
            let offset = |e: &Field, axis: &Field| match e {
                Field::Bin(BinOp::Add, a, c) if std::mem::discriminant(a.as_ref()) == std::mem::discriminant(axis) => {
                    c.constant().map(|c| c * res as f64).filter(|v| (v - v.round()).abs() < 1e-9).map(|v| v.round() as i64)
                }
                _ => None,
            };
            if let Field::State(k) = inner.as_ref() {
                if let (Some(di), Some(dj)) = (offset(x, &Field::X), offset(y, &Field::Y)) {
                    return Field::Grid { data: cur[*k].clone(), res, di, dj };
                }
            }
            let key = Arc::as_ptr(inner);
            let inner = match memo.get(&key) {
                Some(d) => d.clone(),
                None => {
                    let d = Arc::new(bind_state(inner, cur, res, memo));
                    memo.insert(key, d.clone());
                    d
                }
            };
            let (x, y) = (bind_state(x, cur, res, memo), bind_state(y, cur, res, memo));
            Field::Remap { inner, x: Box::new(x), y: Box::new(y) }
        }
        Field::Reseed { inner, seed } => Field::Reseed { inner: Arc::new(bind_state(inner, cur, res, memo)), seed: *seed },
        Field::Raster { img, cx, cy, width, angle } => {
            Field::Raster { img: img.clone(), cx: b(cx), cy: b(cy), width: b(width), angle: b(angle) }
        }
        other => other.clone(),
    }
}

pub const MOTION: &[&str] = &["at", "ease", "keys", "progress", "sequence"];
pub const FILTERS: &[&str] = &["blur", "pixelate", "edges", "relief", "posterize", "film", "vignette", "bloom", "chroma"];
pub const EASINGS: &[&str] = &["linear", "in", "out", "in-out", "sine", "back", "elastic", "bounce"];

fn add(a: Field, b: Field) -> Field {
    fold(Field::Bin(BinOp::Add, Box::new(a), Box::new(b)))
}
fn sub(a: Field, b: Field) -> Field {
    fold(Field::Bin(BinOp::Sub, Box::new(a), Box::new(b)))
}
fn mul(a: Field, b: Field) -> Field {
    fold(Field::Bin(BinOp::Mul, Box::new(a), Box::new(b)))
}
fn call(b: Builtin, args: Vec<Field>) -> Field {
    fold(Field::Call(b, args))
}

/// From 0 (before `start`) to 1 (after `end`), in a straight line between.
fn progress_field(start: Field, end: Field, time: Field) -> Field {
    let span = share_field(sub(end, start.clone()));
    let safe = call(Builtin::Max, vec![span, Field::Const(1e-9)]);
    call(Builtin::Clamp, vec![fold(Field::Bin(BinOp::Div, Box::new(sub(time, start)), Box::new(safe))), Field::Const(0.0), Field::Const(1.0)])
}

/// An easing curve, from 0 to 1 as `u` goes from 0 to 1.
fn ease_field(u: Field, kind: &str) -> Field {
    let c = Field::Const;
    let u = share_field(call(Builtin::Clamp, vec![u, c(0.0), c(1.0)]));
    let cube = |v: Field| { let v = share_field(v); mul(mul(v.clone(), v.clone()), v) };
    let square = |v: Field| { let v = share_field(v); mul(v.clone(), v) };
    match kind {
        "linear" => u,
        "in" => cube(u),
        "out" => sub(c(1.0), cube(sub(c(1.0), u))),
        "in-out" => call(Builtin::Smoothstep, vec![c(0.0), c(1.0), u]),
        "sine" => sub(c(0.5), mul(c(0.5), call(Builtin::Cos, vec![mul(c(PI), u)]))),
        "back" => {
            // overshoots a little, then settles
            let (c1, c3) = (1.70158, 2.70158);
            let v = share_field(sub(u, c(1.0)));
            add(c(1.0), add(mul(c(c3), cube(v.clone())), mul(c(c1), square(v))))
        }
        "elastic" => {
            // 2^(-10u) · sin((10u − 0.75) · 2π/3) + 1, exactly 0 at the start
            let decay = call(Builtin::Exp, vec![mul(c(-10.0 * std::f64::consts::LN_2), u.clone())]);
            let wave = call(Builtin::Sin, vec![mul(sub(mul(c(10.0), u.clone()), c(0.75)), c(TAU / 3.0))]);
            let e = add(mul(decay, wave), c(1.0));
            // the formula is not exactly 0 at u = 0 in floating point: hold it
            mul(e, call(Builtin::Step, vec![c(1e-12), u]))
        }
        _ => {
            // bounce: four parabolas touching 1; the lowest one is the ball
            let n1 = 7.5625;
            let d1 = 2.75;
            let arc = |shift: f64, top: f64| add(mul(c(n1), square(sub(u.clone(), c(shift / d1)))), c(top));
            let a = mul(c(n1), square(u.clone()));
            call(Builtin::Min, vec![call(Builtin::Min, vec![a, arc(1.5, 0.75)]), call(Builtin::Min, vec![arc(2.25, 0.9375), arc(2.625, 0.984375)])])
        }
    }
}

/// Applies `f` to a value, or to every value of a list.
fn map_fields(v: Val, span: Span, f: &dyn Fn(Field) -> Field) -> Res<Val> {
    match v {
        Val::Field(x) => Ok(share(Val::Field(f(x)))),
        Val::List(items) => Ok(Val::List(Arc::new(items.iter().map(|i| map_fields(i.clone(), span, f)).collect::<Res<Vec<_>>>()?))),
        other => Err(Error::new(format!("expected a value or a list, found {}", kind_of(&other)), span)),
    }
}

/// Matches arguments to parameter names: unnamed ones first, in order, then named ones.
fn bind_slots<'a>(name: &str, names: &[&str], args: &'a [Arg]) -> Res<Vec<Option<&'a Expr>>> {
    let mut slots: Vec<Option<&Expr>> = vec![None; names.len()];
    let mut named_seen = false;
    for (k, a) in args.iter().enumerate() {
        let slot = match &a.name {
            None if named_seen => return Err(Error::new("unnamed arguments must come before named ones", a.val.span)),
            None => k,
            Some(n) => {
                named_seen = true;
                names.iter().position(|p| p == n).ok_or_else(|| {
                    Error::new(format!("`{name}` has no parameter `{n}` — it has: {}", names.join(", ")), a.val.span)
                })?
            }
        };
        if slot >= names.len() {
            return Err(Error::new(format!("`{name}` takes at most {} arguments: {}", names.len(), names.join(", ")), a.val.span));
        }
        if slots[slot].is_some() {
            return Err(Error::new(format!("`{}` given twice", names[slot]), a.val.span));
        }
        slots[slot] = Some(&a.val);
    }
    Ok(slots)
}

fn share_field(f: Field) -> Field {
    match share(Val::Field(f)) {
        Val::Field(f) => f,
        _ => unreachable!(),
    }
}

fn point_of(v: &Val, span: Span) -> Res<(Field, Field)> {
    match v {
        Val::List(p) if p.len() == 2 => match (&p[0], &p[1]) {
            (Val::Field(x), Val::Field(y)) => Ok((x.clone(), y.clone())),
            _ => Err(Error::new("a point is two values, [x, y]", span)),
        },
        other => Err(Error::new(format!("expected a point [x, y], found {}", kind_of(other)), span)),
    }
}

/// The length of each segment of a path, the whole length, and where each
/// segment starts (the sum of the ones before).
fn path_lengths_full(pts: &[(Field, Field)]) -> (Vec<Field>, Field, Vec<Field>) {
    let mut lengths = Vec::new();
    let mut starts = Vec::new();
    let mut c = Field::Const(0.0);
    for w in pts.windows(2) {
        let l = share_field(fold(Field::Call(Builtin::Distance, vec![w[0].0.clone(), w[0].1.clone(), w[1].0.clone(), w[1].1.clone()])));
        starts.push(c.clone());
        c = share_field(fold(Field::Bin(BinOp::Add, Box::new(c), Box::new(l.clone()))));
        lengths.push(l);
    }
    (lengths, c, starts)
}

fn path_lengths(pts: &[(Field, Field)]) -> (Vec<Field>, Field) {
    let (l, total, _) = path_lengths_full(pts);
    (l, total)
}

/// Arithmetic on lists works element by element; a single value applies to
/// every element.
fn bin_val(op: BinOp, a: Val, b: Val, span: Span) -> Res<Val> {
    match (a, b) {
        (Val::Field(x), Val::Field(y)) => Ok(Val::Field(fold(Field::Bin(op, Box::new(x), Box::new(y))))),
        (Val::List(x), Val::List(y)) => {
            if x.len() != y.len() {
                return Err(Error::new(format!("these lists have different lengths ({} and {})", x.len(), y.len()), span));
            }
            let v = x.iter().zip(y.iter()).map(|(p, q)| bin_val(op, p.clone(), q.clone(), span)).collect::<Res<Vec<_>>>()?;
            Ok(Val::List(Arc::new(v)))
        }
        (Val::List(x), y @ Val::Field(_)) => {
            Ok(Val::List(Arc::new(x.iter().map(|p| bin_val(op, p.clone(), y.clone(), span)).collect::<Res<Vec<_>>>()?)))
        }
        (x @ Val::Field(_), Val::List(y)) => {
            Ok(Val::List(Arc::new(y.iter().map(|q| bin_val(op, x.clone(), q.clone(), span)).collect::<Res<Vec<_>>>()?)))
        }
        (x, y) => {
            let bad = if matches!(x, Val::Field(_) | Val::List(_)) { y } else { x };
            Err(Error::new(format!("{} can't be used in a computation", kind_of(&bad)), span))
        }
    }
}

fn neg_val(v: Val, span: Span) -> Res<Val> {
    match v {
        Val::Field(f) => Ok(Val::Field(fold(Field::Neg(Box::new(f))))),
        Val::List(items) => Ok(Val::List(Arc::new(items.iter().map(|i| neg_val(i.clone(), span)).collect::<Res<Vec<_>>>()?))),
        other => Err(Error::new(format!("{} can't be used in a computation", kind_of(&other)), span)),
    }
}

/// A function on values; given lists, it applies element by element.
fn call_val(b: Builtin, vs: &[Val], span: Span) -> Res<Val> {
    let len = vs.iter().find_map(|v| if let Val::List(l) = v { Some(l.len()) } else { None });
    match len {
        None => {
            let mut fs = Vec::with_capacity(vs.len());
            for v in vs {
                match v {
                    Val::Field(f) => fs.push(f.clone()),
                    Val::Dist(_) => return Err(Error::new("this is a distribution, not a value: put `~` in front of it to draw one", span)),
                    other => return Err(Error::new(format!("{} can't be used in a computation", kind_of(other)), span)),
                }
            }
            Ok(Val::Field(fold(Field::Call(b, fs))))
        }
        Some(n) => {
            let mut out = Vec::with_capacity(n);
            for k in 0..n {
                let mut row = Vec::with_capacity(vs.len());
                for v in vs {
                    row.push(match v {
                        Val::List(l) if l.len() == n => l[k].clone(),
                        Val::List(l) => {
                            return Err(Error::new(format!("these lists have different lengths ({n} and {})", l.len()), span));
                        }
                        other => other.clone(),
                    });
                }
                out.push(call_val(b, &row, span)?);
            }
            Ok(Val::List(Arc::new(out)))
        }
    }
}

/// sum, mean, minimum, maximum of a list (of numbers or fields); stddev of numbers.
fn reduce_list(name: &str, items: &[Val], span: Span) -> Res<Val> {
    if items.is_empty() {
        return match name {
            "sum" => Ok(Val::Field(Field::Const(0.0))),
            _ => Err(Error::new(format!("`{name}` of an empty list"), span)),
        };
    }
    let mut acc: Option<Val> = None;
    for v in items {
        acc = Some(match acc {
            None => v.clone(),
            Some(a) => match name {
                "sum" | "mean" | "stddev" => bin_val(BinOp::Add, a, v.clone(), span)?,
                "minimum" => call_val(Builtin::Min, &[a, v.clone()], span)?,
                _ => call_val(Builtin::Max, &[a, v.clone()], span)?,
            },
        });
    }
    let acc = acc.unwrap();
    let n = Val::Field(Field::Const(items.len() as f64));
    match name {
        "mean" => bin_val(BinOp::Div, acc, n, span),
        "stddev" => {
            let mean = bin_val(BinOp::Div, acc, n.clone(), span)?;
            let mut sq: Option<Val> = None;
            for v in items {
                let d = bin_val(BinOp::Sub, v.clone(), mean.clone(), span)?;
                let d2 = bin_val(BinOp::Mul, d.clone(), d, span)?;
                sq = Some(match sq {
                    None => d2,
                    Some(s) => bin_val(BinOp::Add, s, d2, span)?,
                });
            }
            let var = bin_val(BinOp::Div, sq.unwrap(), n, span)?;
            call_val(Builtin::Sqrt, &[var], span)
        }
        _ => Ok(acc),
    }
}

/// Constant folding: `2 * 3` becomes `6`, `sin(~uniform(…))` becomes a number.
pub(crate) fn fold(f: Field) -> Field {
    let all_const = match &f {
        Field::Neg(a) => a.constant().is_some(),
        Field::Bin(_, a, b) => a.constant().is_some() && b.constant().is_some(),
        Field::Call(b, args) => *b != Builtin::Noise && args.iter().all(|a| a.constant().is_some()),
        _ => false,
    };
    if all_const { Field::Const(f.eval([0.0; 3], 0)) } else { f }
}

/// Samples a field on a grid, for the aggregates used in constraints.
fn samples(f: &Field, seed: u64) -> Vec<f64> {
    const N: usize = 24;
    let mut v = Vec::with_capacity(N * N * 4);
    for k in 0..4 {
        for j in 0..N {
            for i in 0..N {
                let p = [(i as f64 + 0.5) / N as f64, (j as f64 + 0.5) / N as f64, k as f64 / 3.0];
                let x = f.eval(p, seed);
                if x.is_finite() {
                    v.push(x);
                }
            }
        }
    }
    v
}

// ───────────────────────────── values & editions ─────────────────────────────

#[derive(Clone, Debug)]
pub enum Val {
    Field(Field),
    Str(String),
    Dist(Dist),
    /// `[a, b, c]`; a point is a list of two numbers.
    List(Arc<Vec<Val>>),
    /// `let f(a, b) = …`
    Func(Arc<FuncDef>),
    /// A layer of sound: `drone(…)`, `notes(…)`, `texture(…)`.
    Sound(Arc<crate::sound::Layer>),
    /// A field in colour, or paintings laid out: `paint`, `mosaic`, `place`, `overlay`.
    Paint(Arc<crate::paint::Paint>),
}

/// A function written in alea: its parameters, its body, and the names it
/// could see where it was written.
#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Expr,
    pub env: HashMap<String, Val>,
}

const MAX_LIST: usize = 100_000;

fn kind_of(v: &Val) -> &'static str {
    match v {
        Val::Field(_) => "a value",
        Val::Str(_) => "a text",
        Val::Dist(_) => "a distribution",
        Val::List(_) => "a list",
        Val::Func(_) => "a function",
        Val::Sound(_) => "a sound",
        Val::Paint(_) => "a painting",
    }
}

/// Formats a number for a text: whole numbers without decimals, padded with
/// zeros to `pad` digits; others with up to six decimals.
fn format_number(v: f64, pad: usize) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        let n = v as i64;
        let digits = format!("{}", n.abs());
        let digits = if digits.len() < pad { format!("{}{digits}", "0".repeat(pad - digits.len())) } else { digits };
        if n < 0 { format!("-{digits}") } else { digits }
    } else {
        let s = format!("{v:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug)]
pub struct Render {
    pub kind: String,
    pub span: Span,
    pub positional: Vec<(Val, Span)>,
    pub named: HashMap<String, (Val, Span)>,
}

/// An edition: the work once drawn. Nothing random remains in it except
/// what is derived deterministically from `seed`.
#[derive(Debug)]
pub struct Edition {
    pub name: String,
    pub seed: u64,
    pub attempts: u32,
    pub renders: Vec<Render>,
}

pub fn resolve(module: &Module, seed: u64) -> Res<Edition> {
    let o = draw_work(module, seed, &HashMap::new(), false, 0)?;
    Ok(Edition { name: module.work.name.clone(), seed, attempts: o.attempts, renders: o.renders })
}

struct Outcome {
    renders: Vec<Render>,
    output: Option<Val>,
    attempts: u32,
}

/// Draws one edition of a work. When the work is a component (called from
/// another work), its renders are skipped and its constraints are retried
/// locally: only this component is redrawn, not the piece around it.
fn draw_work(m: &Module, seed: u64, given: &HashMap<String, Val>, component: bool, depth: usize) -> Res<Outcome> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut failures: Vec<(Span, u32)> = Vec::new();
    for attempt in 1..=MAX_ATTEMPTS {
        let mut r = Resolver { rng: &mut rng, env: HashMap::new(), next_id: 0, seed, module: m, depth, grid: None };
        match r.run(&m.work, given, component)? {
            Ok((renders, output)) => return Ok(Outcome { renders, output, attempts: attempt }),
            Err(span) => match failures.iter_mut().find(|(s, _)| *s == span) {
                Some((_, n)) => *n += 1,
                None => failures.push((span, 1)),
            },
        }
    }
    let (span, _) = failures.into_iter().max_by_key(|(_, n)| *n).unwrap();
    Err(Error::new(
        format!("constraint never satisfied in {MAX_ATTEMPTS} attempts — the space of possibilities may be empty"),
        span,
    ))
}

pub(crate) struct Resolver<'a> {
    pub(crate) rng: &'a mut ChaCha8Rng,
    env: HashMap<String, Val>,
    pub(crate) next_id: u64,
    seed: u64,
    pub(crate) module: &'a Module,
    depth: usize,
    /// Inside a `grow` rule: the grid's cells per side (for `neighbours` & co).
    grid: Option<usize>,
}

fn reseed(v: Val, seed: u64) -> Val {
    match v {
        Val::Field(f) if f.constant().is_none() => Val::Field(Field::Reseed { inner: Arc::new(f), seed }),
        Val::List(items) => Val::List(Arc::new(items.iter().map(|i| reseed(i.clone(), seed)).collect())),
        other => other,
    }
}

/// Shares a non-constant field behind an `Arc` so references stay cheap.
fn share(v: Val) -> Val {
    match v {
        Val::Field(f) if f.constant().is_none() && !matches!(f, Field::Ref(_) | Field::Pattern(_)) => {
            Val::Field(Field::Ref(Arc::new(f)))
        }
        Val::Field(f) => match f.constant() {
            Some(k) => Val::Field(Field::Const(k)),
            None => Val::Field(f),
        },
        Val::List(items) => Val::List(Arc::new(items.iter().map(|i| share(i.clone())).collect())),
        other => other,
    }
}

impl Resolver<'_> {
    /// `Ok(Err(span))` means the constraint at `span` failed: draw again.
    #[allow(clippy::type_complexity)]
    fn run(
        &mut self,
        w: &Work,
        given: &HashMap<String, Val>,
        component: bool,
    ) -> Res<Result<(Vec<Render>, Option<Val>), Span>> {
        for p in &w.params {
            let v = match (given.get(&p.name), &p.default) {
                (Some(v), _) => v.clone(),
                (None, Some(d)) => share(self.lower(d)?),
                (None, None) => {
                    return Err(Error::new(
                        format!(
                            "parameter `{}` has no default value, so this work can't run on its own — \
                             give it one (`{} = …`) or use this work from another one",
                            p.name, p.name
                        ),
                        p.span,
                    ));
                }
            };
            self.env.insert(p.name.clone(), v);
        }
        let mut renders = Vec::new();
        let mut output: Option<Val> = None;
        for s in &w.stmts {
            match s {
                Stmt::Output { val, span } => {
                    if output.is_some() {
                        return Err(Error::new("a work has only one `output`", *span));
                    }
                    let v = share(self.lower(val)?);
                    if !matches!(v, Val::Field(_) | Val::Paint(_)) {
                        return Err(Error::new("`output` must be a shape, texture, field, pattern or painting", val.span));
                    }
                    self.env.insert("output".into(), v.clone());
                    output = Some(v);
                }
                Stmt::Render { .. } if component => {}
                Stmt::Let { name, val, .. } => {
                    let v = self.lower(val)?;
                    self.env.insert(name.clone(), share(v));
                }
                Stmt::Func { name, params, body, .. } => {
                    // It sees the names given so far, not itself: no endless calls.
                    let def = FuncDef { name: name.clone(), params: params.clone(), body: body.clone(), env: self.env.clone() };
                    self.env.insert(name.clone(), Val::Func(Arc::new(def)));
                }
                Stmt::Field { name, val, .. } => {
                    let f = self.field(val)?;
                    self.env.insert(name.clone(), share(Val::Field(f)));
                }
                Stmt::Pattern { name, count, attrs, .. } => {
                    let evs = self.pattern(count, attrs)?;
                    self.env.insert(name.clone(), Val::Field(Field::Pattern(Arc::new(evs))));
                }
                Stmt::Grow { size, steps, rules, span } => self.grow(size.as_ref(), steps.as_ref(), rules, *span)?,
                Stmt::Constraint { val, span } => {
                    let v = self.scalar(val, "a constraint")?;
                    if v == 0.0 || v.is_nan() {
                        return Ok(Err(*span));
                    }
                }
                Stmt::Render { kind, args, span } => {
                    if !RENDER_KINDS.contains(&kind.as_str()) {
                        return Err(Error::new(
                            format!("unknown render kind `{kind}` — available: {}", RENDER_KINDS.join(", ")),
                            *span,
                        ));
                    }
                    let mut positional = Vec::new();
                    let mut named = HashMap::new();
                    for a in args {
                        let v = self.lower(&a.val)?;
                        match &a.name {
                            None => positional.push((v, a.val.span)),
                            Some(n) => {
                                if named.insert(n.clone(), (v, a.val.span)).is_some() {
                                    return Err(Error::new(format!("argument `{n}` given twice"), a.val.span));
                                }
                            }
                        }
                    }
                    renders.push(Render { kind: kind.clone(), span: *span, positional, named });
                }
            }
        }
        if renders.is_empty() && !component {
            return Err(Error::bare(if output.is_some() {
                "this work has an `output` but no `render`: it is meant to be used by another work — \
                 to run it on its own, add `render image(output)`"
            } else {
                "the work contains no `render`: nothing to produce"
            }));
        }
        Ok(Ok((renders, output)))
    }

    /// `Moon(size: 0.3)`: draws a fresh edition of another work and returns its output.
    fn call_work(&mut self, m: &Arc<Module>, args: &[Arg], span: Span) -> Res<Val> {
        let w = &m.work;
        if self.depth >= MAX_DEPTH {
            return Err(Error::new(format!("works are nested more than {MAX_DEPTH} levels deep"), span));
        }
        if !w.stmts.iter().any(|s| matches!(s, Stmt::Output { .. })) {
            return Err(Error::new(
                format!("`{}` has no `output`, so it can't be used inside another work — add `output …` to it", w.name),
                span,
            ));
        }
        let names: Vec<&str> = w.params.iter().map(|p| p.name.as_str()).collect();
        let mut given: HashMap<String, Val> = HashMap::new();
        let mut named_seen = false;
        for (k, a) in args.iter().enumerate() {
            let name = match &a.name {
                None if named_seen => {
                    return Err(Error::new("unnamed arguments must come before named ones", a.val.span));
                }
                None => match names.get(k) {
                    Some(n) => n.to_string(),
                    None => {
                        let list = if names.is_empty() { "none".to_string() } else { names.join(", ") };
                        return Err(Error::new(
                            format!("`{}` takes {} parameter(s): {list}", w.name, names.len()),
                            a.val.span,
                        ));
                    }
                },
                Some(n) => {
                    named_seen = true;
                    if !names.contains(&n.as_str()) {
                        let list = if names.is_empty() { "none".to_string() } else { names.join(", ") };
                        return Err(Error::new(
                            format!("`{}` has no parameter `{n}` — it has: {list}", w.name),
                            a.val.span,
                        ));
                    }
                    n.clone()
                }
            };
            // A field handed over keeps the caller's seed, so it looks the same inside.
            let v = reseed(share(self.lower(&a.val)?), self.seed);
            if given.insert(name.clone(), v).is_some() {
                return Err(Error::new(format!("parameter `{name}` given twice"), a.val.span));
            }
        }
        if let Some(p) = w.params.iter().find(|p| p.default.is_none() && !given.contains_key(&p.name)) {
            return Err(Error::new(format!("`{}` needs a value for `{}`", w.name, p.name), span));
        }
        // A new draw on every call, yet fully determined by the caller's seed.
        let child_seed = self.rng.random::<u64>();
        let o = draw_work(m, child_seed, &given, true, self.depth + 1).map_err(|e| e.in_file(&m.file, &m.src))?;
        Ok(reseed(o.output.expect("checked above"), child_seed))
    }

    /// `grow over N for S { name = start then next … }`
    fn grow(&mut self, size: Option<&Expr>, steps: Option<&Expr>, rules: &[GrowRule], span: Span) -> Res<()> {
        let whole = |v: f64, lo: f64, hi: f64| v.fract() == 0.0 && v >= lo && v <= hi;
        let res = match size {
            Some(e) => {
                let v = self.scalar(e, "the size of a grow grid")?;
                if !whole(v, 4.0, 1024.0) {
                    return Err(Error::new("`over`: the grid has a whole number of cells per side, from 4 to 1024", e.span));
                }
                v as usize
            }
            None => 128,
        };
        let steps = match steps {
            Some(e) => {
                let v = self.scalar(e, "the number of generations")?;
                if !whole(v, 1.0, 100_000.0) {
                    return Err(Error::new("`for`: a whole number of generations, from 1 to 100 000", e.span));
                }
                v as usize
            }
            None => 200,
        };
        for (k, r) in rules.iter().enumerate() {
            if rules[..k].iter().any(|o| o.name == r.name) {
                return Err(Error::new(format!("`{}` is grown twice in this block", r.name), r.span));
            }
        }
        // Where each field starts: computed before any of them exists.
        let mut starts = Vec::new();
        for r in rules {
            starts.push(self.field(&r.start)?);
        }
        // How each becomes the next: its own name, and its siblings', are the
        // previous generation.
        let saved: Vec<Option<Val>> =
            rules.iter().enumerate().map(|(k, r)| self.env.insert(r.name.clone(), Val::Field(Field::State(k)))).collect();
        let outer = self.grid.replace(res);
        let mut nexts = Vec::new();
        let mut failed = None;
        for r in rules {
            match self.field(&r.next) {
                Ok(f) => nexts.push(f),
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }
        self.grid = outer;
        for (r, old) in rules.iter().zip(saved) {
            match old {
                Some(v) => self.env.insert(r.name.clone(), v),
                None => self.env.remove(&r.name),
            };
        }
        if let Some(e) = failed {
            return Err(e);
        }
        let _ = span;
        let data = Arc::new(simulate(&starts, &nexts, res, steps, self.seed));
        for (k, r) in rules.iter().enumerate() {
            self.env.insert(r.name.clone(), Val::Field(Field::Grow { data: data.clone(), k }));
        }
        Ok(())
    }

    /// `neighbours(f)`, `spread(f)`, `nearby(f, r)`: what is around each cell of a grow grid.
    fn grid_helper(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        let Some(res) = self.grid else {
            return Err(Error::new(
                format!("`{name}` looks at the neighbouring cells of a grid: it only exists in a `grow` rule, after `then`"),
                span,
            ));
        };
        if let Some(a) = args.iter().find(|a| a.name.is_some()) {
            return Err(Error::new(format!("`{name}` takes no named argument"), a.val.span));
        }
        let want = if name == "nearby" { 2 } else { 1 };
        if args.len() != want {
            return Err(Error::new(format!("`{name}` expects {want} argument(s), got {}", args.len()), span));
        }
        let f = self.field(&args[0].val)?;
        // A grown name stays bare, so its neighbours can be read cell by cell.
        let f = Arc::new(match f {
            Field::Ref(r) if matches!(r.as_ref(), Field::State(_)) => r.as_ref().clone(),
            other => other,
        });
        let d = 1.0 / res as f64;
        let at = |dx: i32, dy: i32| -> Field {
            if dx == 0 && dy == 0 {
                return Field::Ref(f.clone());
            }
            Field::Remap {
                inner: f.clone(),
                x: Box::new(Field::Bin(BinOp::Add, Box::new(Field::X), Box::new(Field::Const(dx as f64 * d)))),
                y: Box::new(Field::Bin(BinOp::Add, Box::new(Field::Y), Box::new(Field::Const(dy as f64 * d)))),
            }
        };
        let mut terms: Vec<(f64, Field)> = Vec::new();
        match name {
            "neighbours" | "neighbors" => {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx != 0 || dy != 0 {
                            terms.push((1.0, at(dx, dy)));
                        }
                    }
                }
            }
            "spread" => {
                // The discrete Laplacian: how much the surroundings exceed the cell.
                terms.push((-1.0, at(0, 0)));
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    terms.push((0.2, at(dx, dy)));
                }
                for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                    terms.push((0.05, at(dx, dy)));
                }
            }
            _ => {
                let r = self.scalar(&args[1].val, "the radius of `nearby`")?;
                if !(r.fract() == 0.0 && (1.0..=12.0).contains(&r)) {
                    return Err(Error::new("`nearby`: the radius is a whole number of cells, from 1 to 12", args[1].val.span));
                }
                let r = r as i32;
                let mut cells = Vec::new();
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dy * dy <= r * r {
                            cells.push((dx, dy));
                        }
                    }
                }
                let w = 1.0 / cells.len() as f64;
                for (dx, dy) in cells {
                    terms.push((w, at(dx, dy)));
                }
            }
        }
        let mut sum: Option<Field> = None;
        for (w, t) in terms {
            let term = if w == 1.0 { t } else { Field::Bin(BinOp::Mul, Box::new(Field::Const(w)), Box::new(t)) };
            sum = Some(match sum {
                None => term,
                Some(acc) => Field::Bin(BinOp::Add, Box::new(acc), Box::new(term)),
            });
        }
        Ok(Val::Field(sum.expect("at least one cell")))
    }

    pub(crate) fn lower(&mut self, e: &Expr) -> Res<Val> {
        Ok(match &e.kind {
            ExprKind::Num(n) => Val::Field(Field::Const(*n)),
            ExprKind::Str(s) => Val::Str(s.clone()),
            ExprKind::Var(v) => match v.as_str() {
                "x" => Val::Field(Field::X),
                "y" => Val::Field(Field::Y),
                "t" => Val::Field(Field::T),
                "pi" => Val::Field(Field::Const(PI)),
                "tau" => Val::Field(Field::Const(TAU)),
                "i" | "n" if !self.env.contains_key(v) => {
                    return Err(Error::new(format!("`{v}` only exists inside a pattern"), e.span));
                }
                _ => self
                    .env
                    .get(v)
                    .cloned()
                    .ok_or_else(|| Error::new(format!("unknown name `{v}`"), e.span))?,
            },
            ExprKind::Neg(a) => {
                let v = self.lower(a)?;
                neg_val(v, a.span)?
            }
            ExprKind::Bin(op, a, b) => {
                let (va, vb) = (self.lower(a)?, self.lower(b)?);
                bin_val(*op, va, vb, e.span)?
            }
            ExprKind::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for i in items {
                    v.push(share(self.lower(i)?));
                }
                Val::List(Arc::new(v))
            }
            ExprKind::Comprehension { body, loops } => {
                let mut out = Vec::new();
                self.comprehension(body, loops, &mut out, e.span)?;
                Val::List(Arc::new(out))
            }
            ExprKind::Index(base, i) => {
                let items = self.list(base)?;
                let k = self.scalar(i, "an index")?;
                if k.fract() != 0.0 || !k.is_finite() {
                    return Err(Error::new("an index is a whole number: 0 is the first element, -1 the last", i.span));
                }
                let n = items.len() as i64;
                let j = if k < 0.0 { n + k as i64 } else { k as i64 };
                if j < 0 || j >= n {
                    return Err(Error::new(format!("index {k} is outside this list of {n} element(s)"), i.span));
                }
                items[j as usize].clone()
            }
            ExprKind::Member(base, name) => {
                let k = match name.as_str() {
                    "x" => 0,
                    "y" => 1,
                    _ => return Err(Error::new(format!("`.{name}`: a point has `.x` and `.y` (for anything else, use [index])"), e.span)),
                };
                let items = self.list(base)?;
                if items.len() < 2 {
                    return Err(Error::new(format!("`.{name}` applies to a point, a list of two numbers [x, y]"), base.span));
                }
                items[k].clone()
            }
            ExprKind::Template(parts) => {
                let mut text = String::new();
                for part in parts {
                    match part {
                        TextPart::Lit(l) => text.push_str(l),
                        TextPart::Value(v, pad) => match self.lower(v)? {
                            Val::Str(s) => text.push_str(&s),
                            Val::Field(f) => match f.constant() {
                                Some(c) => text.push_str(&format_number(c, *pad)),
                                None => return Err(Error::new("a value inside a text must be a fixed number, not a field", v.span)),
                            },
                            other => return Err(Error::new(format!("{} can't go inside a text", kind_of(&other)), v.span)),
                        },
                    }
                }
                Val::Str(text)
            }
            ExprKind::Draw { dist, per_point } => {
                let Val::Dist(d) = self.lower(dist)? else {
                    return Err(Error::new(
                        "`~` applies to a distribution (uniform, normal, beta, poisson, exponential)",
                        dist.span,
                    ));
                };
                if *per_point {
                    self.next_id += 1;
                    Val::Field(Field::Grain { dist: d, id: self.next_id })
                } else {
                    Val::Field(Field::Const(d.draw(&mut *self.rng)))
                }
            }
            ExprKind::Choose(branches) => {
                let mut weights = Vec::with_capacity(branches.len());
                for (w, _) in branches {
                    let x = self.scalar(w, "a weight")?;
                    if !(x >= 0.0 && x.is_finite()) {
                        return Err(Error::new("a weight must be zero or positive", w.span));
                    }
                    weights.push(x);
                }
                let total: f64 = weights.iter().sum();
                if total <= 0.0 {
                    return Err(Error::new("the weights must sum to a positive number", e.span));
                }
                let mut u = self.rng.random::<f64>() * total;
                let mut k = branches.len() - 1;
                for (j, w) in weights.iter().enumerate() {
                    if u < *w {
                        k = j;
                        break;
                    }
                    u -= w;
                }
                self.lower(&branches[k].1)?
            }
            ExprKind::Call(name, args) => self.call(name, args, e.span)?,
        })
    }

    pub(crate) fn field(&mut self, e: &Expr) -> Res<Field> {
        match self.lower(e)? {
            Val::Field(f) => Ok(f),
            Val::List(_) => Err(Error::new(
                "a list can't be used here: take one element (`list[0]`, `point.x`) or combine them (`merge(list)`, `sum(list)`)",
                e.span,
            )),
            Val::Func(f) => Err(Error::new(format!("`{}` is a function: call it, `{}(…)`", f.name, f.name), e.span)),
            Val::Sound(_) => Err(Error::new("a sound can't be used in a computation: give it to `render sound(…)` or a video's `sound:`", e.span)),
            Val::Paint(_) => Err(Error::new(
                "a painting is already in colour, so it can't be used in a computation: lay it out with `mosaic`, `place` or `overlay`, or render it",
                e.span,
            )),
            Val::Str(_) => Err(Error::new("a string cannot be used in a computation", e.span)),
            Val::Dist(_) => Err(Error::new(
                "this is a distribution, not a value: put `~` in front of it to draw one",
                e.span,
            )),
        }
    }

    pub(crate) fn scalar(&mut self, e: &Expr, what: &str) -> Res<f64> {
        self.field(e)?.constant().ok_or_else(|| {
            Error::new(
                format!("{what} must be a fixed value, but this expression depends on x, y, t or a `~~` grain"),
                e.span,
            )
        })
    }

    fn call(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        if let Some(Val::Func(f)) = self.env.get(name).cloned() {
            return self.call_func(&f, args, span);
        }
        if let Some(m) = self.module.deps.get(name).cloned() {
            return self.call_work(&m, args, span);
        }
        if let Some(r) = self.list_builtin(name, args, span) {
            return r;
        }
        if matches!(name, "drone" | "notes" | "texture") {
            return self.sound_layer(name, args, span);
        }
        if matches!(name, "paint" | "mosaic" | "place" | "overlay") {
            return self.painting(name, args, span);
        }
        if matches!(name, "neighbours" | "neighbors" | "spread" | "nearby") {
            return self.grid_helper(name, args, span);
        }
        if MOTION.contains(&name) || FILTERS.contains(&name) {
            return self.motion(name, args, span);
        }
        if let Some(r) = self.primitive(name, args, span) {
            return r;
        }
        if let Some(a) = args.iter().find(|a| a.name.is_some()) {
            return Err(Error::new(format!("`{name}` takes no named argument"), a.val.span));
        }
        let arity = |n: usize| -> Res<()> {
            if args.len() != n {
                Err(Error::new(format!("`{name}` expects {n} argument(s), got {}", args.len()), span))
            } else {
                Ok(())
            }
        };
        match name {
            "uniform" | "normal" | "beta" | "poisson" | "exponential" | "between" | "around" => {
                let name = match name {
                    "between" => "uniform",
                    "around" => "normal",
                    n => n,
                };
                arity(if matches!(name, "poisson" | "exponential") { 1 } else { 2 })?;
                let mut v = Vec::new();
                for a in args {
                    v.push(self.scalar(&a.val, "a distribution parameter")?);
                }
                let invalid = |msg: &str| Err(Error::new(format!("{name}: {msg}"), span));
                let d = match name {
                    "uniform" => Dist::Uniform(v[0], v[1]),
                    "normal" if !(v[1] > 0.0) => return invalid("the standard deviation must be > 0"),
                    "normal" => Dist::Normal(v[0], v[1]),
                    "beta" if !(v[0] > 0.0 && v[1] > 0.0) => return invalid("both parameters must be > 0"),
                    "beta" => Dist::Beta(v[0], v[1]),
                    "poisson" if !(v[0] > 0.0) => return invalid("λ must be > 0"),
                    "poisson" => Dist::Poisson(v[0]),
                    "exponential" if !(v[0] > 0.0) => return invalid("λ must be > 0"),
                    _ => Dist::Exponential(v[0]),
                };
                Ok(Val::Dist(d))
            }
            "mean" | "stddev" | "minimum" | "maximum" => {
                arity(1)?;
                let v = self.lower(&args[0].val)?;
                if let Val::List(items) = v {
                    return reduce_list(name, &items, args[0].val.span);
                }
                let Val::Field(f) = v else { return Err(Error::new(format!("`{name}` applies to a field or a list"), args[0].val.span)) };
                let v = samples(&f, self.seed);
                let n = v.len().max(1) as f64;
                let mean = v.iter().sum::<f64>() / n;
                let r = match name {
                    "mean" => mean,
                    "stddev" => (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt(),
                    "minimum" => v.iter().copied().fold(f64::INFINITY, f64::min),
                    _ => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                };
                Ok(Val::Field(Field::Const(r)))
            }
            "count" => {
                arity(1)?;
                let v = self.lower(&args[0].val)?;
                if let Val::List(items) = &v {
                    return Ok(Val::Field(Field::Const(items.len() as f64)));
                }
                match &v {
                    Val::Field(f) if f.pattern().is_some() => Ok(Val::Field(Field::Const(f.pattern().unwrap().len() as f64))),
                    _ => Err(Error::new("`count` applies to a pattern or a list", args[0].val.span)),
                }
            }
            _ => match Builtin::lookup(name) {
                Some((Builtin::Distance, _)) if args.len() == 2 => {
                    // distance(p, q) between two points
                    let (p, q) = (self.point(&args[0].val)?, self.point(&args[1].val)?);
                    Ok(Val::Field(fold(Field::Call(Builtin::Distance, vec![p.0, p.1, q.0, q.1]))))
                }
                Some((b, n)) => {
                    arity(n)?;
                    let vs = args.iter().map(|a| self.lower(&a.val)).collect::<Res<Vec<_>>>()?;
                    call_val(b, &vs, span)
                }
                None => Err(Error::new(format!("unknown function `{name}`"), span)),
            },
        }
    }

    fn list(&mut self, e: &Expr) -> Res<Arc<Vec<Val>>> {
        match self.lower(e)? {
            Val::List(items) => Ok(items),
            other => Err(Error::new(format!("expected a list, found {}", kind_of(&other)), e.span)),
        }
    }

    /// A point: a list of two values, [x, y].
    fn point(&mut self, e: &Expr) -> Res<(Field, Field)> {
        let items = self.list(e)?;
        point_of(&Val::List(items), e.span)
    }

    fn points(&mut self, e: &Expr, at_least: usize) -> Res<Vec<(Field, Field)>> {
        let items = self.list(e)?;
        if items.len() < at_least {
            return Err(Error::new(format!("this needs at least {at_least} points, got {}", items.len()), e.span));
        }
        items.iter().map(|p| point_of(p, e.span)).collect()
    }

    fn comprehension(&mut self, body: &Expr, loops: &[(String, Expr)], out: &mut Vec<Val>, span: Span) -> Res<()> {
        let Some(((name, iter), rest)) = loops.split_first() else {
            if out.len() >= MAX_LIST {
                return Err(Error::new(format!("a list has at most {MAX_LIST} elements"), span));
            }
            out.push(share(self.lower(body)?));
            return Ok(());
        };
        let items = self.list(iter)?;
        let saved = self.env.get(name).cloned();
        let mut result = Ok(());
        for item in items.iter() {
            self.env.insert(name.clone(), item.clone());
            result = self.comprehension(body, rest, out, span);
            if result.is_err() {
                break;
            }
        }
        match saved {
            Some(v) => self.env.insert(name.clone(), v),
            None => self.env.remove(name),
        };
        result
    }

    /// Calls a function written in alea: the arguments are computed where the
    /// call is, the body where the function was written.
    fn call_func(&mut self, f: &Arc<FuncDef>, args: &[Arg], span: Span) -> Res<Val> {
        let names: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
        let mut given: Vec<Option<Val>> = vec![None; names.len()];
        let mut named_seen = false;
        for (k, a) in args.iter().enumerate() {
            let slot = match &a.name {
                None if named_seen => return Err(Error::new("unnamed arguments must come before named ones", a.val.span)),
                None => k,
                Some(n) => {
                    named_seen = true;
                    names.iter().position(|p| p == n).ok_or_else(|| {
                        Error::new(format!("`{}` has no parameter `{n}` — it has: {}", f.name, names.join(", ")), a.val.span)
                    })?
                }
            };
            if slot >= names.len() {
                return Err(Error::new(format!("`{}` takes {} argument(s): {}", f.name, names.len(), names.join(", ")), a.val.span));
            }
            if given[slot].is_some() {
                return Err(Error::new(format!("`{}` given twice", names[slot]), a.val.span));
            }
            given[slot] = Some(share(self.lower(&a.val)?));
        }
        let outer = std::mem::replace(&mut self.env, f.env.clone());
        let result = (|| {
            for (p, v) in f.params.iter().zip(given) {
                let v = match (v, &p.default) {
                    (Some(v), _) => v,
                    (None, Some(d)) => share(self.lower(d)?),
                    (None, None) => return Err(Error::new(format!("`{}` needs a value for `{}`", f.name, p.name), span)),
                };
                self.env.insert(p.name.clone(), v);
            }
            self.lower(&f.body)
        })();
        self.env = outer;
        result.map(share)
    }

    /// A painting from a value: a painting as it is, a field in "ink".
    fn as_paint(&mut self, v: Val, span: Span) -> Res<Arc<crate::paint::Paint>> {
        match v {
            Val::Paint(p) => Ok(p),
            Val::Field(f) => Ok(Arc::new(crate::paint::Paint::Field {
                field: f,
                seed: self.seed,
                stops: crate::media::Palette::named("ink").expect("built in"),
                range: None,
                alpha: None,
                bounds: std::sync::OnceLock::new(),
            })),
            other => Err(Error::new(format!("expected a painting or a field, found {}", kind_of(&other)), span)),
        }
    }

    /// `paint(field, …)`, `mosaic(list, …)`, `place(painting, …)`, `overlay(…)`.
    fn painting(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        use crate::paint::*;
        let names: &[&str] = match name {
            "paint" => &["field", "palette", "range", "alpha", "bands"],
            "mosaic" => &["paintings", "columns", "gap", "background"],
            "place" => &["painting", "x", "y", "width", "height"],
            _ => &["paintings", "blend", "opacity", "mask"],
        };
        // overlay(a, b, c, …): every unnamed argument is a layer
        let mut layers: Vec<&Expr> = Vec::new();
        let mut slots: Vec<Option<&Expr>> = vec![None; names.len()];
        let mut named_seen = false;
        for (k, a) in args.iter().enumerate() {
            match &a.name {
                None if named_seen => return Err(Error::new("unnamed arguments must come before named ones", a.val.span)),
                None if name == "overlay" => layers.push(&a.val),
                None if k < names.len() => slots[k] = Some(&a.val),
                None => return Err(Error::new(format!("`{name}` takes at most {} arguments: {}", names.len(), names.join(", ")), a.val.span)),
                Some(n) => {
                    named_seen = true;
                    let k = names.iter().position(|p| p == n).ok_or_else(|| {
                        Error::new(format!("`{name}` has no parameter `{n}` — it has: {}", names.join(", ")), a.val.span)
                    })?;
                    slots[k] = Some(&a.val);
                }
            }
        }
        let num = |r: &mut Self, k: usize, default: f64| -> Res<f64> {
            match slots[k] {
                None => Ok(default),
                Some(e) => r.scalar(e, names[k]),
            }
        };
        let paint = match name {
            "paint" => {
                let Some(e) = slots[0] else { return Err(Error::new("`paint` needs a field", span)) };
                let v = self.lower(e)?;
                if let Val::Paint(p) = v {
                    return Ok(Val::Paint(p));
                }
                let field = match v {
                    Val::Field(f) => f,
                    other => return Err(Error::new(format!("`paint` colours a field, not {}", kind_of(&other)), e.span)),
                };
                let bands = match slots[4] {
                    None => 0,
                    Some(e) => {
                        let b = self.scalar(e, "bands")?;
                        if !(b >= 2.0 && b <= 256.0 && b.fract() == 0.0) {
                            return Err(Error::new("`bands` is a whole number of flat levels, from 2 to 256", e.span));
                        }
                        b as usize
                    }
                };
                let stops = match slots[1] {
                    None => crate::media::Palette { bands, ..crate::media::Palette::named("ink").expect("built in") },
                    Some(e) => {
                        let v = self.lower(e)?;
                        crate::media::palette_from(&v, e.span, bands)?
                    }
                };
                let range = match slots[2] {
                    None => None,
                    Some(e) => {
                        let items = self.list(e)?;
                        let fixed: Vec<Option<f64>> = items.iter().map(|v| if let Val::Field(f) = v { f.constant() } else { None }).collect();
                        match fixed.as_slice() {
                            [Some(lo), Some(hi)] => Some((*lo, *hi)),
                            _ => return Err(Error::new("`range` is two fixed numbers, [low, high]", e.span)),
                        }
                    }
                };
                let alpha = match slots[3] {
                    None => None,
                    Some(e) => Some(self.field(e)?),
                };
                Paint::Field { field, seed: self.seed, stops, range, alpha, bounds: std::sync::OnceLock::new() }
            }
            "mosaic" => {
                let Some(e) = slots[0] else { return Err(Error::new("`mosaic` needs a list of paintings", span)) };
                let items = self.list(e)?;
                if items.is_empty() {
                    return Err(Error::new("`mosaic` of an empty list", e.span));
                }
                let cells = items.iter().map(|v| self.as_paint(v.clone(), e.span)).collect::<Res<Vec<_>>>()?;
                let default_cols = (cells.len() as f64).sqrt().ceil();
                let columns = num(self, 1, default_cols)?;
                if !(columns >= 1.0 && columns.fract() == 0.0) {
                    return Err(Error::new("`columns` is a whole number, 1 or more", slots[1].map_or(span, |e| e.span)));
                }
                let gap = num(self, 2, 0.01)?.clamp(0.0, 0.25);
                let background = match slots[3] {
                    None => Arc::new(Paint::Flat([0.0; 3])),
                    Some(e) => match self.lower(e)? {
                        Val::Field(f) if f.constant().is_some() => {
                            let g = f.constant().unwrap().clamp(0.0, 1.0) as f32;
                            Arc::new(Paint::Flat([g; 3]))
                        }
                        other => self.as_paint(other, e.span)?,
                    },
                };
                Paint::Mosaic { cells, columns: columns as usize, gap, background }
            }
            "place" => {
                let Some(e) = slots[0] else { return Err(Error::new("`place` needs a painting", span)) };
                let v = self.lower(e)?;
                let inner = self.as_paint(v, e.span)?;
                let (x, y) = (num(self, 1, 0.5)?, num(self, 2, 0.5)?);
                let width = num(self, 3, 0.5)?;
                let height = num(self, 4, width)?;
                if !(width > 0.0 && height > 0.0) {
                    return Err(Error::new("`place`: the width and height must be more than 0", span));
                }
                Paint::Place { inner, x, y, width, height }
            }
            _ => {
                let mut ps = Vec::new();
                for e in layers {
                    match self.lower(e)? {
                        Val::List(items) => {
                            for v in items.iter() {
                                ps.push(self.as_paint(v.clone(), e.span)?);
                            }
                        }
                        v => ps.push(self.as_paint(v, e.span)?),
                    }
                }
                if ps.is_empty() {
                    return Err(Error::new("`overlay` needs paintings: `overlay(below, above, …)`", span));
                }
                let blend = match slots[1] {
                    None => Blend::Normal,
                    Some(e) => match self.lower(e)? {
                        Val::Str(s) if Blend::parse(&s).is_some() => Blend::parse(&s).unwrap(),
                        _ => return Err(Error::new(format!("`blend` is one of: {}", BLENDS.join(", ")), e.span)),
                    },
                };
                let opacity = num(self, 2, 1.0)?.clamp(0.0, 1.0);
                let mask = match slots[3] {
                    None => None,
                    Some(e) => Some((self.field(e)?, self.seed)),
                };
                Paint::Overlay { layers: ps, blend, opacity, mask }
            }
        };
        Ok(Val::Paint(Arc::new(paint)))
    }

    /// Time (`at`, `ease`, `keys`, `progress`, `sequence`) and filters
    /// (`blur`, `pixelate`, `edges`, `relief`, `posterize`, `film`,
    /// `vignette`, `bloom`, `chroma`).
    fn motion(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        use crate::paint::{FilterKind, Paint};
        let names: &[&str] = match name {
            "at" => &["of", "time"],
            "ease" => &["value", "kind"],
            "keys" => &["frames", "ease", "time"],
            "progress" => &["start", "end", "time"],
            "sequence" => &["items", "fade", "time"],
            "blur" => &["of", "radius"],
            "pixelate" => &["of", "size"],
            "edges" => &["of", "step"],
            "relief" => &["of", "angle", "depth", "step"],
            "posterize" => &["of", "levels"],
            _ => &["of", "amount"],
        };
        let slots = bind_slots(name, names, args)?;
        let Some(first) = slots[0] else {
            return Err(Error::new(format!("`{name}` needs {}", names[0]), span));
        };
        let num = |r: &mut Self, k: usize, default: f64| -> Res<f64> {
            match slots[k] {
                None => Ok(default),
                Some(e) => r.scalar(e, &format!("`{}`", names[k])),
            }
        };
        let time = |r: &mut Self, k: usize| -> Res<Field> {
            match slots[k] {
                None => Ok(Field::T),
                Some(e) => r.field(e),
            }
        };
        let kind = |r: &mut Self, k: usize, default: &str| -> Res<String> {
            match slots[k] {
                None => Ok(default.to_string()),
                Some(e) => match r.lower(e)? {
                    Val::Str(s) if EASINGS.contains(&s.as_str()) => Ok(s),
                    _ => Err(Error::new(format!("an easing is one of: {}", EASINGS.join(", ")), e.span)),
                },
            }
        };
        let field_only = |what: &str| {
            Error::new(
                format!("`{name}` works on a field, not on a painting: apply it before `paint` ({what})"),
                first.span,
            )
        };
        let shared = |f: Field| -> Arc<Field> {
            match f {
                Field::Ref(r) => r,
                other => Arc::new(other),
            }
        };
        match name {
            "at" => {
                let when = time(self, 1)?;
                if slots[1].is_none() {
                    return Err(Error::new("`at(work, time)` needs the time to show", span));
                }
                match self.lower(first)? {
                    Val::Field(f) => Ok(Val::Field(Field::Retime { inner: shared(f), time: Box::new(when) })),
                    Val::Paint(p) => Ok(Val::Paint(Arc::new(Paint::Retime { inner: p, time: when, seed: self.seed }))),
                    other => Err(Error::new(format!("`at` shows a field or a painting at another time, not {}", kind_of(&other)), first.span)),
                }
            }
            "ease" => {
                let k = kind(self, 1, "in-out")?;
                map_fields(self.lower(first)?, first.span, &|f| ease_field(f, &k))
            }
            "progress" => {
                let (a, b) = (self.field(first)?, match slots[1] {
                    Some(e) => self.field(e)?,
                    None => return Err(Error::new("`progress(start, end)` needs an end", span)),
                });
                Ok(Val::Field(progress_field(a, b, time(self, 2)?)))
            }
            "keys" => {
                let frames = self.list(first)?;
                if frames.len() < 2 {
                    return Err(Error::new("`keys` needs at least two frames, [[time, value], [time, value], …]", first.span));
                }
                let mut keyed: Vec<(f64, Val)> = Vec::new();
                for fr in frames.iter() {
                    let bad = || Error::new("each frame is [time, value], with a fixed time from 0 to 1", first.span);
                    let Val::List(pair) = fr else { return Err(bad()) };
                    if pair.len() != 2 {
                        return Err(bad());
                    }
                    let Val::Field(tf) = &pair[0] else { return Err(bad()) };
                    let tk = tf.constant().ok_or_else(bad)?;
                    if let Some((prev, _)) = keyed.last() {
                        if tk < *prev {
                            return Err(Error::new("the frames of `keys` go in order of time", first.span));
                        }
                    }
                    keyed.push((tk, pair[1].clone()));
                }
                let k = kind(self, 1, "linear")?;
                let when = share_field(time(self, 2)?);
                let mut acc = keyed[0].1.clone();
                for w in keyed.windows(2) {
                    let ((t0, v0), (t1, v1)) = (&w[0], &w[1]);
                    let u = ease_field(progress_field(Field::Const(*t0), Field::Const(*t1), when.clone()), &k);
                    let step = bin_val(BinOp::Sub, v1.clone(), v0.clone(), first.span)?;
                    let step = bin_val(BinOp::Mul, step, Val::Field(u), first.span)?;
                    acc = bin_val(BinOp::Add, acc, step, first.span)?;
                }
                Ok(share(acc))
            }
            "sequence" => {
                let items = self.list(first)?;
                if items.is_empty() {
                    return Err(Error::new("`sequence` of an empty list", first.span));
                }
                let fade = num(self, 1, 0.1)?.clamp(0.0, 1.0 / items.len() as f64);
                let when = share_field(time(self, 2)?);
                if items.iter().any(|v| matches!(v, Val::Paint(_))) {
                    let ps = items.iter().map(|v| self.as_paint(v.clone(), first.span)).collect::<Res<Vec<_>>>()?;
                    return Ok(Val::Paint(Arc::new(Paint::Sequence { items: ps, fade, time: when, seed: self.seed })));
                }
                let n = items.len() as f64;
                let mut acc: Option<Field> = None;
                for (i, v) in items.iter().enumerate() {
                    let Val::Field(f) = v else {
                        return Err(Error::new(format!("`sequence` shows fields or paintings, not {}", kind_of(v)), first.span));
                    };
                    let (start, end) = (i as f64 / n, (i + 1) as f64 / n);
                    let h = (fade / 2.0).max(1e-9);
                    let rise = progress_field(Field::Const(start - h), Field::Const(start + h), when.clone());
                    let fall = progress_field(Field::Const(end - h), Field::Const(end + h), when.clone());
                    let mut wt = Field::Const(1.0);
                    if i > 0 {
                        wt = rise;
                    }
                    if i + 1 < items.len() {
                        let down = sub(Field::Const(1.0), fall);
                        wt = if i > 0 { call(Builtin::Min, vec![wt, down]) } else { down };
                    }
                    let term = mul(wt, f.clone());
                    acc = Some(match acc {
                        None => term,
                        Some(a) => add(a, term),
                    });
                }
                Ok(Val::Field(acc.expect("not empty")))
            }
            _ => {
                let v = self.lower(first)?;
                let filter = |kind: FilterKind, r: &Self, p: Arc<Paint>| Val::Paint(Arc::new(Paint::Filter { inner: p, kind, seed: r.seed }));
                match (name, v) {
                    ("blur", Val::Paint(p)) => Ok(filter(FilterKind::Blur(num(self, 1, 0.01)?.clamp(0.0, 0.3)), self, p)),
                    ("pixelate", Val::Paint(p)) => Ok(filter(FilterKind::Pixelate(num(self, 1, 0.02)?.clamp(0.0, 0.5)), self, p)),
                    ("film", Val::Paint(p)) => Ok(filter(FilterKind::Film(num(self, 1, 0.08)?.clamp(0.0, 1.0)), self, p)),
                    ("vignette", Val::Paint(p)) => Ok(filter(FilterKind::Vignette(num(self, 1, 0.5)?.clamp(0.0, 1.0)), self, p)),
                    ("bloom", v) => {
                        let p = self.as_paint(v, first.span)?;
                        Ok(filter(FilterKind::Bloom(num(self, 1, 0.8)?.clamp(0.0, 4.0)), self, p))
                    }
                    ("chroma", v) => {
                        let p = self.as_paint(v, first.span)?;
                        Ok(filter(FilterKind::Chroma(num(self, 1, 0.5)?.clamp(0.0, 4.0)), self, p))
                    }
                    ("edges" | "relief" | "posterize", Val::Paint(_)) => Err(field_only("it reads the values of the field")),
                    (_, Val::Field(f)) => {
                        let f = shared(f);
                        let at = |dx: Field, dy: Field| Field::Remap { inner: f.clone(), x: Box::new(add(Field::X, dx)), y: Box::new(add(Field::Y, dy)) };
                        let c = Field::Const;
                        Ok(Val::Field(match name {
                            "blur" => {
                                let r = num(self, 1, 0.01)?.max(0.0);
                                if r == 0.0 {
                                    return Ok(Val::Field(Field::Ref(f)));
                                }
                                // 40 taps on a sunflower spiral, out to twice the radius,
                                // weighted like a Gaussian bell (the spiral leaves no rings)
                                let n = 40;
                                let taps: Vec<(f64, f64, f64)> = (0..n)
                                    .map(|k| {
                                        let d = 2.0 * r * ((k as f64 + 0.5) / n as f64).sqrt();
                                        let a = k as f64 * 2.399_963_229_728_653;
                                        (d * a.cos(), d * a.sin(), (-(d / r).powi(2) / 2.0).exp())
                                    })
                                    .collect();
                                let total: f64 = taps.iter().map(|t| t.2).sum();
                                let mut acc: Option<Field> = None;
                                for (dx, dy, w) in taps {
                                    let term = mul(c(w / total), if dx == 0.0 && dy == 0.0 { Field::Ref(f.clone()) } else { at(c(dx), c(dy)) });
                                    acc = Some(match acc {
                                        None => term,
                                        Some(a) => add(a, term),
                                    });
                                }
                                acc.unwrap()
                            }
                            "pixelate" => {
                                let s = num(self, 1, 0.02)?;
                                if !(s > 0.0) {
                                    return Err(Error::new("`pixelate`: the size must be more than 0", span));
                                }
                                let cell = |v: Field| mul(add(call(Builtin::Floor, vec![mul(v, c(1.0 / s))]), c(0.5)), c(s));
                                Field::Remap { inner: f.clone(), x: Box::new(cell(Field::X)), y: Box::new(cell(Field::Y)) }
                            }
                            "edges" => {
                                let h = num(self, 1, 0.002)?.max(1e-6);
                                let gx = sub(at(c(h), c(0.0)), at(c(-h), c(0.0)));
                                let gy = sub(at(c(0.0), c(h)), at(c(0.0), c(-h)));
                                let (gx, gy) = (share_field(gx), share_field(gy));
                                call(Builtin::Sqrt, vec![add(mul(gx.clone(), gx), mul(gy.clone(), gy))])
                            }
                            "relief" => {
                                let a = num(self, 1, 135.0)?.to_radians();
                                let depth = num(self, 2, 10.0)?;
                                let h = num(self, 3, 0.003)?.max(1e-6);
                                let (dx, dy) = (a.cos() * h, -a.sin() * h);
                                add(c(0.5), mul(c(depth), sub(at(c(dx), c(dy)), at(c(-dx), c(-dy)))))
                            }
                            "posterize" => {
                                let n = num(self, 1, 6.0)?;
                                if !(n >= 2.0 && n.fract() == 0.0) {
                                    return Err(Error::new("`posterize`: the levels are a whole number, 2 or more", span));
                                }
                                let v = call(Builtin::Clamp, vec![Field::Ref(f.clone()), c(0.0), c(0.99999)]);
                                mul(call(Builtin::Floor, vec![mul(v, c(n))]), c(1.0 / (n - 1.0)))
                            }
                            "film" => {
                                let amount = num(self, 1, 0.08)?;
                                self.next_id += 1;
                                add(Field::Ref(f.clone()), mul(c(amount), Field::Grain { dist: Dist::Uniform(-1.0, 1.0), id: self.next_id }))
                            }
                            "vignette" => {
                                let amount = num(self, 1, 0.5)?;
                                let r = call(Builtin::Distance, vec![Field::X, Field::Y, c(0.5), c(0.5)]);
                                let e = call(Builtin::Smoothstep, vec![c(0.25), c(0.75), r]);
                                mul(Field::Ref(f.clone()), sub(c(1.0), mul(c(amount), e)))
                            }
                            _ => unreachable!(),
                        }))
                    }
                    (_, other) => Err(Error::new(format!("`{name}` filters a field or a painting, not {}", kind_of(&other)), first.span)),
                }
            }
        }
    }

    /// `drone(field, …)`, `notes(pattern, …)`, `texture(field, …)`: layers of sound.
    fn sound_layer(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        use crate::sound::*;
        let names: &[&str] = match name {
            "drone" => &["field", "voices", "instrument", "scale", "key", "gain"],
            "notes" => &["pattern", "instrument", "scale", "key", "gain", "tempo"],
            _ => &["field", "kind", "gain"],
        };
        let mut slots: Vec<Option<&Expr>> = vec![None; names.len()];
        let mut named_seen = false;
        for (k, a) in args.iter().enumerate() {
            let slot = match &a.name {
                None if named_seen => return Err(Error::new("unnamed arguments must come before named ones", a.val.span)),
                None => k,
                Some(n) => {
                    named_seen = true;
                    names.iter().position(|p| p == n).ok_or_else(|| {
                        Error::new(format!("`{name}` has no parameter `{n}` — it has: {}", names.join(", ")), a.val.span)
                    })?
                }
            };
            if slot >= names.len() {
                return Err(Error::new(format!("`{name}` takes at most {} arguments: {}", names.len(), names.join(", ")), a.val.span));
            }
            slots[slot] = Some(&a.val);
        }
        let Some(first) = slots[0] else {
            return Err(Error::new(format!("`{name}` needs {}", if name == "notes" { "a pattern" } else { "a field" }), span));
        };
        let text = |r: &mut Self, k: usize, default: &str, allowed: &[&str]| -> Res<String> {
            match slots[k] {
                None => Ok(default.to_string()),
                Some(e) => match r.lower(e)? {
                    Val::Str(s) if allowed.contains(&s.as_str()) => Ok(s),
                    _ => Err(Error::new(format!("`{}` is one of: {}", names[k], allowed.join(", ")), e.span)),
                },
            }
        };
        let number = |r: &mut Self, k: usize, default: f64, lo: f64, hi: f64| -> Res<f64> {
            match slots[k] {
                None => Ok(default),
                Some(e) => {
                    let v = r.scalar(e, names[k])?;
                    if !(lo..=hi).contains(&v) {
                        return Err(Error::new(format!("`{}` goes from {lo} to {hi}", names[k]), e.span));
                    }
                    Ok(v)
                }
            }
        };
        let scale_of = |s: String| Scale::parse(&s).expect("checked");
        let layer = match name {
            "drone" => {
                let field = self.field(first)?;
                let voices = number(self, 1, 6.0, 1.0, 24.0)?.round() as usize;
                let voice = Voice::parse(&text(self, 2, "sine", DRONE_INSTRUMENTS)?).expect("checked");
                let scale = scale_of(text(self, 3, "pentatonic", SCALES)?);
                let key = number(self, 4, 110.0, 20.0, 2000.0)?;
                let gain = number(self, 5, 1.0, 0.0, 10.0)?;
                Layer::Drone { field, seed: self.seed, voices, voice, scale, key, gain }
            }
            "notes" => {
                let f = self.field(first)?;
                let Some(events) = f.pattern().cloned() else {
                    return Err(Error::new("`notes` plays a pattern (`pattern p = repeat …`)", first.span));
                };
                let strike = Strike::parse(&text(self, 1, "bell", NOTE_INSTRUMENTS)?).expect("checked");
                let scale = scale_of(text(self, 2, "pentatonic", SCALES)?);
                let key = number(self, 3, 220.0, 20.0, 4000.0)?;
                let gain = number(self, 4, 1.0, 0.0, 10.0)?;
                let tempo = number(self, 5, 0.0, 0.0, 400.0)?;
                Layer::Notes { events, strike, scale, key, gain, tempo, seed: self.seed }
            }
            _ => {
                let field = self.field(first)?;
                let kind = Ambience::parse(&text(self, 1, "wind", TEXTURES)?).expect("checked");
                let gain = number(self, 2, 1.0, 0.0, 10.0)?;
                Layer::Texture { field, seed: self.seed, kind, gain }
            }
        };
        Ok(Val::Sound(Arc::new(layer)))
    }

    /// Functions on lists: range, join, slice, reverse, sum, and paths of points.
    fn list_builtin(&mut self, name: &str, args: &[Arg], span: Span) -> Option<Res<Val>> {
        if !matches!(name, "range" | "join" | "slice" | "reverse" | "sum" | "path" | "along" | "length") {
            return None;
        }
        Some((|| {
            let positional = |n: usize| -> Res<()> {
                if let Some(a) = args.iter().find(|a| a.name.is_some()) {
                    return Err(Error::new(format!("`{name}` takes no named argument"), a.val.span));
                }
                if args.len() != n {
                    return Err(Error::new(format!("`{name}` expects {n} argument(s), got {}", args.len()), span));
                }
                Ok(())
            };
            let whole = |v: f64, e: &Expr| -> Res<i64> {
                if v.fract() != 0.0 || !v.is_finite() {
                    return Err(Error::new(format!("`{name}` expects whole numbers"), e.span));
                }
                Ok(v as i64)
            };
            match name {
                "range" => {
                    let (a, b) = match args.len() {
                        1 => (0, whole(self.scalar(&args[0].val, "a count")?, &args[0].val)?),
                        2 => (
                            whole(self.scalar(&args[0].val, "a start")?, &args[0].val)?,
                            whole(self.scalar(&args[1].val, "an end")?, &args[1].val)?,
                        ),
                        _ => return Err(Error::new("`range(n)` gives 0 to n − 1; `range(a, b)` gives a to b − 1", span)),
                    };
                    if b - a > MAX_LIST as i64 {
                        return Err(Error::new(format!("a list has at most {MAX_LIST} elements"), span));
                    }
                    Ok(Val::List(Arc::new((a..b.max(a)).map(|k| Val::Field(Field::Const(k as f64))).collect())))
                }
                "join" => {
                    let mut out = Vec::new();
                    for a in args {
                        match self.lower(&a.val)? {
                            Val::List(items) => out.extend(items.iter().cloned()),
                            other => out.push(share(other)),
                        }
                    }
                    Ok(Val::List(Arc::new(out)))
                }
                "slice" => {
                    positional(3)?;
                    let items = self.list(&args[0].val)?;
                    let n = items.len() as i64;
                    let mut ends = [0i64; 2];
                    for (k, a) in args[1..].iter().enumerate() {
                        let v = whole(self.scalar(&a.val, "a position")?, &a.val)?;
                        ends[k] = (if v < 0 { n + v } else { v }).clamp(0, n);
                    }
                    let (from, to) = (ends[0], ends[1].max(ends[0]));
                    Ok(Val::List(Arc::new(items[from as usize..to as usize].to_vec())))
                }
                "reverse" => {
                    positional(1)?;
                    let items = self.list(&args[0].val)?;
                    Ok(Val::List(Arc::new(items.iter().rev().cloned().collect())))
                }
                "sum" => {
                    positional(1)?;
                    let items = self.list(&args[0].val)?;
                    reduce_list("sum", &items, args[0].val.span)
                }
                "length" => {
                    positional(1)?;
                    let pts = self.points(&args[0].val, 2)?;
                    Ok(Val::Field(path_lengths(&pts).1))
                }
                "along" => {
                    positional(2)?;
                    let pts = self.points(&args[0].val, 2)?;
                    let d = self.field(&args[1].val)?;
                    let (lengths, _, starts) = path_lengths_full(&pts);
                    // how far along each segment d has gone, from 0 (not yet) to 1 (past it)
                    let (mut px, mut py) = (pts[0].0.clone(), pts[0].1.clone());
                    for k in 0..lengths.len() {
                        let u = fold(Field::Call(
                            Builtin::Clamp,
                            vec![
                                fold(Field::Bin(
                                    BinOp::Div,
                                    Box::new(fold(Field::Bin(BinOp::Sub, Box::new(d.clone()), Box::new(starts[k].clone())))),
                                    Box::new(fold(Field::Call(Builtin::Max, vec![lengths[k].clone(), Field::Const(0.000001)]))),
                                )),
                                Field::Const(0.0),
                                Field::Const(1.0),
                            ],
                        ));
                        let u = share_field(u);
                        let step = |a: &Field, b: &Field| fold(Field::Bin(BinOp::Sub, Box::new(b.clone()), Box::new(a.clone())));
                        px = fold(Field::Bin(BinOp::Add, Box::new(px), Box::new(fold(Field::Bin(BinOp::Mul, Box::new(step(&pts[k].0, &pts[k + 1].0)), Box::new(u.clone()))))));
                        py = fold(Field::Bin(BinOp::Add, Box::new(py), Box::new(fold(Field::Bin(BinOp::Mul, Box::new(step(&pts[k].1, &pts[k + 1].1)), Box::new(u))))));
                    }
                    Ok(Val::List(Arc::new(vec![share(Val::Field(px)), share(Val::Field(py))])))
                }
                _ => {
                    // path(points, width, softness): segments with round joints
                    let mut pos: Vec<&Expr> = Vec::new();
                    let (mut width, mut soft) = (None, None);
                    for a in args {
                        match a.name.as_deref() {
                            None => pos.push(&a.val),
                            Some("points") => pos.insert(0, &a.val),
                            Some("width") => width = Some(&a.val),
                            Some("softness") => soft = Some(&a.val),
                            Some(n) => return Err(Error::new(format!("`path` has no parameter `{n}` — it has: points, width, softness"), a.val.span)),
                        }
                    }
                    if pos.len() > 3 || pos.is_empty() {
                        return Err(Error::new("`path(points, width, softness)`: give it a list of points", span));
                    }
                    let width = width.or(pos.get(1).copied());
                    let soft = soft.or(pos.get(2).copied());
                    let pts = self.points(pos[0], 2)?;
                    let width = match width { Some(e) => self.field(e)?, None => Field::Const(0.01) };
                    let soft = match soft { Some(e) => self.field(e)?, None => Field::Const(0.002) };
                    let mut acc: Option<Field> = None;
                    for w in pts.windows(2) {
                        let seg = crate::prims::line_field(w[0].0.clone(), w[0].1.clone(), w[1].0.clone(), w[1].1.clone(), width.clone(), soft.clone());
                        acc = Some(match acc {
                            None => seg,
                            Some(a) => crate::prims::max2(a, seg),
                        });
                    }
                    Ok(Val::Field(acc.expect("two points at least")))
                }
            }
        })())
    }

    /// `repeat N event(...)`: attributes are re-evaluated on every repetition,
    /// so each `~` in them is drawn again. `i` and `n` are defined there.
    fn pattern(&mut self, count: &Expr, attrs: &[Arg]) -> Res<Vec<Event>> {
        let n = self.scalar(count, "the repeat count")?;
        if !(0.0..=100_000.0).contains(&n) {
            return Err(Error::new(format!("repeat count out of range ({n:.0}, max 100,000)"), count.span));
        }
        let n = n.round() as usize;
        for a in attrs {
            match &a.name {
                None => {
                    return Err(Error::new(
                        "event attributes are named: x:, y:, t:, intensity:, size:",
                        a.val.span,
                    ));
                }
                Some(k) if !EVENT_ATTRS.contains(&k.as_str()) => {
                    return Err(Error::new(
                        format!("unknown attribute `{k}` — expected: {}", EVENT_ATTRS.join(", ")),
                        a.val.span,
                    ));
                }
                _ => {}
            }
        }
        let mut evs = Vec::with_capacity(n);
        for k in 0..n {
            self.env.insert("i".into(), Val::Field(Field::Const(k as f64)));
            self.env.insert("n".into(), Val::Field(Field::Const(n as f64)));
            let mut e = Event {
                x: self.rng.random(),
                y: self.rng.random(),
                t: self.rng.random(),
                intensity: 1.0,
                size: 0.02,
            };
            for a in attrs {
                let v = self.scalar(&a.val, "an event attribute")?;
                match a.name.as_deref().unwrap() {
                    "x" => e.x = v,
                    "y" => e.y = v,
                    "t" => e.t = v,
                    "intensity" => e.intensity = v,
                    _ => e.size = v.abs().max(1e-4),
                }
            }
            evs.push(e);
        }
        self.env.remove("i");
        self.env.remove("n");
        Ok(evs)
    }
}
