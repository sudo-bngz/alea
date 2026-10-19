//! Artist-level primitives: shapes, textures, transforms and combinators.
//!
//! They are pure sugar: every primitive lowers to an ordinary `Field` tree, so
//! they work in every render (image, sound, sculpture) and any parameter can
//! itself be a field — `circle(size: 0.2 + 0.1 * t)` breathes over time.
//!
//! Conventions, chosen for artists rather than mathematicians:
//! - x, y run from 0 to 1, origin at the top-left; sizes are fractions of the canvas;
//! - angles are in degrees;
//! - shapes are 1 inside and 0 outside, textures range from 0 to 1;
//! - **an omitted parameter is chosen by alea** (a fresh draw per call).

use std::f64::consts::{PI, TAU};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::sync::Arc;

use rand::Rng;

use crate::ast::{Arg, BinOp};
use crate::error::{Error, Res, Span};
use crate::eval::{fold, Builtin, Dist, Event, Field, Resolver, Val};

// ───────────────────────────── field algebra ─────────────────────────────
// Lets the primitives below be written as plain formulas over `Field`.

impl From<f64> for Field {
    fn from(v: f64) -> Self {
        Field::Const(v)
    }
}

fn bin(op: BinOp, a: Field, b: Field) -> Field {
    fold(Field::Bin(op, Box::new(a), Box::new(b)))
}

macro_rules! field_op {
    ($tr:ident, $m:ident, $op:expr) => {
        impl<T: Into<Field>> $tr<T> for Field {
            type Output = Field;
            fn $m(self, rhs: T) -> Field {
                bin($op, self, rhs.into())
            }
        }
    };
}
field_op!(Add, add, BinOp::Add);
field_op!(Sub, sub, BinOp::Sub);
field_op!(Mul, mul, BinOp::Mul);
field_op!(Div, div, BinOp::Div);
field_op!(Rem, rem, BinOp::Mod);

impl Neg for Field {
    type Output = Field;
    fn neg(self) -> Field {
        fold(Field::Neg(Box::new(self)))
    }
}

fn call(b: Builtin, args: Vec<Field>) -> Field {
    fold(Field::Call(b, args))
}

impl Field {
    fn sin(self) -> Field {
        call(Builtin::Sin, vec![self])
    }
    fn cos(self) -> Field {
        call(Builtin::Cos, vec![self])
    }
    fn abs(self) -> Field {
        call(Builtin::Abs, vec![self])
    }
    fn sqrt(self) -> Field {
        call(Builtin::Sqrt, vec![self])
    }
    fn exp(self) -> Field {
        call(Builtin::Exp, vec![self])
    }
    fn fract(self) -> Field {
        call(Builtin::Fract, vec![self])
    }
    fn floor(self) -> Field {
        call(Builtin::Floor, vec![self])
    }
}

fn c(v: f64) -> Field {
    Field::Const(v)
}
fn fx() -> Field {
    Field::X
}
fn fy() -> Field {
    Field::Y
}
fn ft() -> Field {
    Field::T
}
pub(crate) fn max2(a: Field, b: Field) -> Field {
    call(Builtin::Max, vec![a, b])
}
pub(crate) fn min2(a: Field, b: Field) -> Field {
    call(Builtin::Min, vec![a, b])
}
fn smooth(e0: Field, e1: Field, v: Field) -> Field {
    call(Builtin::Smoothstep, vec![e0, e1, v])
}
fn clamp01(v: Field) -> Field {
    call(Builtin::Clamp, vec![v, c(0.0), c(1.0)])
}
fn noise(x: Field, y: Field, z: Field) -> Field {
    call(Builtin::Noise, vec![x, y, z])
}
fn atan2(y: Field, x: Field) -> Field {
    call(Builtin::Atan2, vec![y, x])
}
fn dist_to(cx: Field, cy: Field) -> Field {
    call(Builtin::Distance, vec![fx(), fy(), cx, cy])
}

/// Wraps a field behind an `Arc` so using it several times in a tree is cheap.
fn share(f: Field) -> Field {
    if f.constant().is_some() || matches!(f, Field::Ref(_)) { f } else { Field::Ref(Arc::new(f)) }
}

fn remap(inner: Field, x: Field, y: Field) -> Field {
    if inner.constant().is_some() {
        return inner;
    }
    Field::Remap { inner: Arc::new(inner), x: Box::new(x), y: Box::new(y) }
}

/// 1 inside radius `r`, 0 outside, with a soft edge of half-width `s`.
/// A segment from (x1, y1) to (x2, y2), `width` wide, with round ends.
pub(crate) fn line_field(x1: Field, y1: Field, x2: Field, y2: Field, width: Field, s: Field) -> Field {
    let (bx, by) = (share(x2 - x1.clone()), share(y2 - y1.clone()));
    let (px, py) = (share(fx() - x1), share(fy() - y1));
    let len2 = bx.clone() * bx.clone() + by.clone() * by.clone() + 1e-9;
    let h = share(clamp01((px.clone() * bx.clone() + py.clone() * by.clone()) / len2));
    let (ex, ey) = (share(px - h.clone() * bx), share(py - h * by));
    inside(width * 0.5, s, (ex.clone() * ex + ey.clone() * ey).sqrt())
}

fn inside(r: Field, s: Field, d: Field) -> Field {
    c(1.0) - smooth(r.clone() - s.clone(), r + s, d)
}

/// Coordinate along the direction `angle` (degrees), centred on the canvas.
fn along(angle_rad: Field) -> Field {
    (fx() - 0.5) * angle_rad.clone().cos() + (fy() - 0.5) * angle_rad.sin()
}

// ───────────────────────────── parameters ─────────────────────────────

/// What happens when a parameter is omitted.
#[derive(Clone, Copy)]
enum Def {
    Fixed(f64),
    /// alea draws a value in [lo, hi].
    Rand(f64, f64),
    /// alea draws a whole number in [lo, hi].
    RandInt(i64, i64),
}

const POS: Def = Def::Rand(0.25, 0.75);
const DEG: f64 = PI / 180.0;

fn params(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "circle" => &["x", "y", "size", "softness"],
        "ring" => &["x", "y", "size", "width", "softness"],
        "rect" => &["x", "y", "width", "height", "angle", "softness"],
        "square" => &["x", "y", "size", "angle", "softness"],
        "line" => &["x1", "y1", "x2", "y2", "width", "softness"],
        "blob" => &["x", "y", "size", "wobble", "softness"],
        "stripes" => &["count", "angle", "softness"],
        "waves" => &["count", "angle", "height", "wiggles", "softness"],
        "ripples" => &["x", "y", "count", "softness"],
        "dots" => &["count", "size", "softness"],
        "checker" => &["count"],
        "gradient" => &["angle"],
        "glow" => &["x", "y", "size"],
        "clouds" => &["scale", "detail"],
        "grain" => &["amount"],
        "move" => &["shape", "x", "y"],
        "rotate" => &["shape", "angle", "x", "y"],
        "zoom" => &["shape", "by", "x", "y"],
        "tile" => &["shape", "count"],
        "mirror" => &["shape", "axis"],
        "kaleidoscope" => &["shape", "sides", "x", "y"],
        "warp" => &["shape", "amount", "scale"],
        "cut" => &["shape", "by"],
        "invert" => &["shape"],
        "threshold" => &["shape", "level", "softness"],
        "scatter" => &["count", "size"],
        "text" => &["words", "x", "y", "size", "angle", "font", "align"],
        "picture" => &["file", "x", "y", "size", "angle", "channel"],
        _ => return None,
    })
}

/// Matches call arguments to parameter names: positional first, then named.
fn bind<'e>(prim: &str, names: &[&str], args: &'e [Arg]) -> Res<Vec<Option<&'e Arg>>> {
    let mut slots = vec![None; names.len()];
    let mut named_seen = false;
    for (k, a) in args.iter().enumerate() {
        let idx = match &a.name {
            None if named_seen => {
                return Err(Error::new("unnamed arguments must come before named ones", a.val.span));
            }
            None if k >= names.len() => {
                return Err(Error::new(
                    format!("`{prim}` takes at most {} argument(s): {}", names.len(), names.join(", ")),
                    a.val.span,
                ));
            }
            None => k,
            Some(n) => {
                named_seen = true;
                names.iter().position(|m| m == n).ok_or_else(|| {
                    Error::new(format!("`{prim}` has no parameter `{n}` — it has: {}", names.join(", ")), a.val.span)
                })?
            }
        };
        if slots[idx].is_some() {
            return Err(Error::new(format!("parameter `{}` given twice", names[idx]), a.val.span));
        }
        slots[idx] = Some(a);
    }
    Ok(slots)
}

struct Bound<'e> {
    prim: String,
    span: Span,
    names: &'static [&'static str],
    slots: Vec<Option<&'e Arg>>,
}

impl<'e> Bound<'e> {
    fn get(&self, name: &str) -> Option<&'e Arg> {
        self.slots[self.names.iter().position(|n| *n == name).expect("known parameter")]
    }
}

impl Resolver<'_> {
    fn draw_default(&mut self, d: Def) -> f64 {
        match d {
            Def::Fixed(v) => v,
            Def::Rand(lo, hi) => lo + (hi - lo) * self.rng.random::<f64>(),
            Def::RandInt(lo, hi) => self.rng.random_range(lo..=hi) as f64,
        }
    }

    /// A parameter as a field (it may vary over space and time).
    fn p(&mut self, b: &Bound, name: &str, d: Def) -> Res<Field> {
        match b.get(name) {
            Some(a) => Ok(share(self.field(&a.val)?)),
            None => Ok(c(self.draw_default(d))),
        }
    }

    /// A softness parameter, kept strictly positive so edges never divide by zero.
    fn soft(&mut self, b: &Bound, default: f64) -> Res<Field> {
        let s = self.p(b, "softness", Def::Fixed(default))?;
        Ok(share(max2(s, c(1e-4))))
    }

    /// A parameter that shapes the structure itself, so it must be a fixed whole number.
    fn whole(&mut self, b: &Bound, name: &str, d: Def, lo: usize, hi: usize) -> Res<usize> {
        let v = match b.get(name) {
            Some(a) => self.scalar(&a.val, &format!("`{name}`"))?,
            None => self.draw_default(d),
        };
        if !(v >= lo as f64 && v <= hi as f64) {
            let span = b.get(name).map(|a| a.val.span).unwrap_or(b.span);
            return Err(Error::new(format!("`{name}` must be between {lo} and {hi}"), span));
        }
        Ok(v.round() as usize)
    }

    fn required(&mut self, b: &Bound, name: &str) -> Res<Field> {
        match b.get(name) {
            Some(a) => Ok(share(self.field(&a.val)?)),
            None => Err(Error::new(format!("`{}` needs a `{name}` to work on", b.prim), b.span)),
        }
    }

    /// A text argument (in quotes), with its position for error messages.
    fn string(&mut self, b: &Bound, name: &str, default: Option<&str>) -> Res<(String, Span)> {
        match b.get(name) {
            Some(a) => match self.lower(&a.val)? {
                Val::Str(s) => Ok((s, a.val.span)),
                _ => Err(Error::new(format!("`{name}` must be text in quotes, like \"…\""), a.val.span)),
            },
            None => match default {
                Some(d) => Ok((d.to_string(), b.span)),
                None => Err(Error::new(format!("`{}` needs its `{name}`, in quotes", b.prim), b.span)),
            },
        }
    }

    fn raster(&mut self, b: &Bound, img: Arc<crate::raster::Raster>, width: Field) -> Res<Field> {
        self.raster_shifted(b, img, width, 0.0)
    }

    /// Placed with its centre `shift` widths along its own direction from
    /// x, y (0.5: x, y is its left end; −0.5: its right end).
    fn raster_shifted(&mut self, b: &Bound, img: Arc<crate::raster::Raster>, width: Field, shift: f64) -> Res<Field> {
        let (x, y) = match b.prim.as_str() {
            "picture" => (Def::Fixed(0.5), Def::Fixed(0.5)),
            _ => (POS, POS),
        };
        let mut cx = self.p(b, "x", x)?;
        let mut cy = self.p(b, "y", y)?;
        let angle = self.p(b, "angle", Def::Fixed(0.0))?;
        if shift != 0.0 {
            let along = |c: Builtin| Field::Call(c, vec![angle.clone() * DEG]);
            let half = width.clone() * shift;
            cx = cx + half.clone() * along(Builtin::Cos);
            cy = cy + half * along(Builtin::Sin);
        }
        Ok(Field::Raster {
            img,
            cx: Box::new(cx),
            cy: Box::new(cy),
            width: Box::new(width),
            angle: Box::new(angle * DEG),
        })
    }

    fn offset(&mut self) -> f64 {
        self.rng.random::<f64>() * 1000.0
    }

    /// Returns `None` when `name` is not a primitive.
    pub(crate) fn primitive(&mut self, name: &str, args: &[Arg], span: Span) -> Option<Res<Val>> {
        if matches!(name, "merge" | "overlap") {
            return Some(self.variadic(name, args, span));
        }
        let names = params(name)?;
        Some(bind(name, names, args).and_then(|slots| {
            let b = Bound { prim: name.to_string(), span, names, slots };
            self.build(&b)
        }))
    }

    fn variadic(&mut self, name: &str, args: &[Arg], span: Span) -> Res<Val> {
        if let Some(a) = args.iter().find(|a| a.name.is_some()) {
            return Err(Error::new(format!("`{name}` takes shapes only, no named parameter"), a.val.span));
        }
        // One list: everything in it.
        if args.len() == 1 {
            if let crate::eval::Val::List(items) = self.lower(&args[0].val)? {
                let mut acc: Option<Field> = None;
                for v in items.iter() {
                    let f = match v {
                        crate::eval::Val::Field(f) => f.clone(),
                        _ => return Err(Error::new(format!("`{name}` of a list: every element must be a shape or a field"), args[0].val.span)),
                    };
                    acc = Some(match acc {
                        None => f,
                        Some(a) => if name == "merge" { max2(a, f) } else { min2(a, f) },
                    });
                }
                return acc
                    .map(crate::eval::Val::Field)
                    .ok_or_else(|| Error::new(format!("`{name}` of an empty list"), args[0].val.span));
            }
        }
        if args.len() < 2 {
            return Err(Error::new(format!("`{name}` needs at least two shapes, or one list of them"), span));
        }
        let mut acc = self.field(&args[0].val)?;
        for a in &args[1..] {
            let f = self.field(&a.val)?;
            acc = if name == "merge" { max2(acc, f) } else { min2(acc, f) };
        }
        Ok(Val::Field(acc))
    }

    fn build(&mut self, b: &Bound) -> Res<Val> {
        let f = match b.prim.as_str() {
            // ── shapes ──
            "circle" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let size = self.p(b, "size", Def::Rand(0.15, 0.45))?;
                let s = self.soft(b, 0.003)?;
                inside(size * 0.5, s, dist_to(cx, cy))
            }
            "ring" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let size = self.p(b, "size", Def::Rand(0.2, 0.6))?;
                let width = self.p(b, "width", Def::Fixed(0.02))?;
                let s = self.soft(b, 0.003)?;
                inside(width * 0.5, s, (dist_to(cx, cy) - size * 0.5).abs())
            }
            "rect" | "square" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let (w, h) = if b.prim == "square" {
                    let size = self.p(b, "size", Def::Rand(0.15, 0.4))?;
                    (size.clone(), size)
                } else {
                    (self.p(b, "width", Def::Rand(0.15, 0.45))?, self.p(b, "height", Def::Rand(0.15, 0.45))?)
                };
                let angle = self.p(b, "angle", Def::Rand(0.0, 90.0))?;
                let s = self.soft(b, 0.003)?;
                let a = share(-angle * DEG);
                let (ca, sa) = (share(a.clone().cos()), share(a.sin()));
                let (dx, dy) = (share(fx() - cx), share(fy() - cy));
                let u = dx.clone() * ca.clone() - dy.clone() * sa.clone();
                let v = dx * sa + dy * ca;
                let sd = max2(u.abs() - w * 0.5, v.abs() - h * 0.5);
                c(1.0) - smooth(-s.clone(), s, sd)
            }
            "line" => {
                let end = Def::Rand(0.1, 0.9);
                let (x1, y1) = (self.p(b, "x1", end)?, self.p(b, "y1", end)?);
                let (x2, y2) = (self.p(b, "x2", end)?, self.p(b, "y2", end)?);
                let width = self.p(b, "width", Def::Fixed(0.01))?;
                let s = self.soft(b, 0.002)?;
                line_field(x1, y1, x2, y2, width, s)
            }
            "blob" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let size = self.p(b, "size", Def::Rand(0.2, 0.45))?;
                let wobble = self.p(b, "wobble", Def::Fixed(0.35))?;
                let s = self.soft(b, 0.003)?;
                let o = self.offset();
                let (dx, dy) = (share(fx() - cx), share(fy() - cy));
                let k = share(c(2.0) / size.clone());
                let n = noise(dx.clone() * k.clone() + o, dy.clone() * k + o, ft() * 0.5 + o);
                let r = size * 0.5 * (c(1.0) + wobble * n);
                inside(r, s, (dx.clone() * dx + dy.clone() * dy).sqrt())
            }

            // ── textures ──
            "stripes" => {
                let count = self.p(b, "count", Def::RandInt(4, 16))?;
                let angle = self.p(b, "angle", Def::Rand(0.0, 180.0))?;
                let s = self.soft(b, 0.3)?;
                smooth(-s.clone(), s, (along(angle * DEG) * count * TAU).sin())
            }
            "waves" => {
                let count = self.p(b, "count", Def::RandInt(4, 12))?;
                let angle = self.p(b, "angle", Def::Rand(0.0, 180.0))?;
                let height = self.p(b, "height", Def::Rand(0.01, 0.04))?;
                let wiggles = self.p(b, "wiggles", Def::RandInt(1, 5))?;
                let s = self.soft(b, 0.3)?;
                let a = share(angle * DEG);
                let u = along(a.clone());
                let v = along(a + PI / 2.0);
                let u = u + height * (v * wiggles * TAU).sin();
                smooth(-s.clone(), s, (u * count * TAU).sin())
            }
            "ripples" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let count = self.p(b, "count", Def::RandInt(5, 20))?;
                let s = self.soft(b, 0.3)?;
                smooth(-s.clone(), s, (dist_to(cx, cy) * count * TAU).sin())
            }
            "dots" => {
                let count = self.p(b, "count", Def::RandInt(5, 15))?;
                let size = self.p(b, "size", Def::Fixed(0.5))?;
                let s = self.soft(b, 0.03)?;
                let gx = share((fx() * count.clone()).fract() - 0.5);
                let gy = share((fy() * count).fract() - 0.5);
                inside(size * 0.5, s, (gx.clone() * gx + gy.clone() * gy).sqrt())
            }
            "checker" => {
                let count = self.p(b, "count", Def::RandInt(4, 12))?;
                ((fx() * count.clone()).floor() + (fy() * count).floor()) % 2.0
            }
            "gradient" => {
                let angle = self.p(b, "angle", Def::Rand(0.0, 360.0))?;
                along(angle * DEG) + 0.5
            }
            "glow" => {
                let (cx, cy) = (self.p(b, "x", POS)?, self.p(b, "y", POS)?);
                let size = self.p(b, "size", Def::Rand(0.2, 0.6))?;
                let d = share(dist_to(cx, cy));
                let sigma = share(size * 0.25);
                (-(d.clone() * d) / (sigma.clone() * sigma * 2.0)).exp()
            }
            "clouds" => {
                let scale = self.p(b, "scale", Def::Rand(2.0, 6.0))?;
                let detail = self.whole(b, "detail", Def::Fixed(4.0), 1, 8)?;
                let (mut sum, mut total) = (c(0.0), 0.0);
                for k in 0..detail {
                    let (amp, freq, o) = (0.5f64.powi(k as i32), 2f64.powi(k as i32), self.offset());
                    let sf = scale.clone() * freq;
                    sum = sum + noise(fx() * sf.clone() + o, fy() * sf + o, ft() + o) * amp;
                    total += amp;
                }
                c(0.5) + sum * (0.5 / total)
            }
            "grain" => {
                let amount = self.p(b, "amount", Def::Fixed(1.0))?;
                self.next_id += 1;
                Field::Grain { dist: Dist::Uniform(0.0, 1.0), id: self.next_id } * amount
            }

            // ── transforms ──
            "move" => {
                let shape = self.required(b, "shape")?;
                let (dx, dy) = (self.p(b, "x", Def::Rand(-0.25, 0.25))?, self.p(b, "y", Def::Rand(-0.25, 0.25))?);
                remap(shape, fx() - dx, fy() - dy)
            }
            "rotate" => {
                let shape = self.required(b, "shape")?;
                let angle = self.p(b, "angle", Def::Rand(0.0, 360.0))?;
                let (cx, cy) = (self.p(b, "x", Def::Fixed(0.5))?, self.p(b, "y", Def::Fixed(0.5))?);
                let a = share(angle * DEG);
                let (ca, sa) = (share(a.clone().cos()), share(a.sin()));
                let (dx, dy) = (share(fx() - cx.clone()), share(fy() - cy.clone()));
                let nx = cx + dx.clone() * ca.clone() + dy.clone() * sa.clone();
                let ny = cy - dx * sa + dy * ca;
                remap(shape, nx, ny)
            }
            "zoom" => {
                let shape = self.required(b, "shape")?;
                let by = self.p(b, "by", Def::Rand(0.5, 2.0))?;
                let (cx, cy) = (self.p(b, "x", Def::Fixed(0.5))?, self.p(b, "y", Def::Fixed(0.5))?);
                let nx = cx.clone() + (fx() - cx) / by.clone();
                let ny = cy.clone() + (fy() - cy) / by;
                remap(shape, nx, ny)
            }
            "tile" => {
                let shape = self.required(b, "shape")?;
                let count = self.p(b, "count", Def::RandInt(2, 6))?;
                remap(shape, (fx() * count.clone()).fract(), (fy() * count).fract())
            }
            "mirror" => {
                let shape = self.required(b, "shape")?;
                let axis = match b.get("axis") {
                    None => "vertical".to_string(),
                    Some(a) => match self.lower(&a.val)? {
                        Val::Str(s) if matches!(s.as_str(), "vertical" | "horizontal" | "both") => s,
                        _ => {
                            return Err(Error::new(
                                "`axis` must be \"vertical\", \"horizontal\" or \"both\"",
                                a.val.span,
                            ));
                        }
                    },
                };
                // The shape together with its reflection(s), so nothing it holds is lost.
                let flip_x = || remap(shape.clone(), c(1.0) - fx(), fy());
                let flip_y = || remap(shape.clone(), fx(), c(1.0) - fy());
                match axis.as_str() {
                    "vertical" => max2(shape.clone(), flip_x()),
                    "horizontal" => max2(shape.clone(), flip_y()),
                    _ => max2(
                        max2(shape.clone(), flip_x()),
                        max2(flip_y(), remap(shape.clone(), c(1.0) - fx(), c(1.0) - fy())),
                    ),
                }
            }
            "kaleidoscope" => {
                let shape = self.required(b, "shape")?;
                let sides = self.p(b, "sides", Def::RandInt(3, 12))?;
                let (cx, cy) = (self.p(b, "x", Def::Fixed(0.5))?, self.p(b, "y", Def::Fixed(0.5))?);
                let (dx, dy) = (share(fx() - cx.clone()), share(fy() - cy.clone()));
                let r = share((dx.clone() * dx.clone() + dy.clone() * dy.clone()).sqrt());
                let seg = share(c(TAU) / sides);
                // Fold every direction onto the wedge centred on "to the right of the centre",
                // so whatever sits there is repeated whole, mirrored on either side.
                let half = share(seg.clone() * 0.5);
                let a = share((((atan2(dy, dx) + half.clone()) % seg) - half).abs());
                remap(shape, cx + r.clone() * a.clone().cos(), cy + r * a.sin())
            }
            "warp" => {
                let shape = self.required(b, "shape")?;
                let amount = self.p(b, "amount", Def::Rand(0.02, 0.08))?;
                let scale = self.p(b, "scale", Def::Rand(2.0, 5.0))?;
                let (o1, o2) = (self.offset(), self.offset());
                let wx = noise(fx() * scale.clone() + o1, fy() * scale.clone() + o1, ft() * 0.5 + o1);
                let wy = noise(fx() * scale.clone() + o2, fy() * scale + o2, ft() * 0.5 + o2);
                remap(shape, fx() + amount.clone() * wx, fy() + amount * wy)
            }

            // ── combining ──
            "cut" => {
                let shape = self.required(b, "shape")?;
                let by = self.required(b, "by")?;
                shape * (c(1.0) - by)
            }
            "invert" => c(1.0) - self.required(b, "shape")?,
            "threshold" => {
                let shape = self.required(b, "shape")?;
                let level = self.p(b, "level", Def::Fixed(0.5))?;
                let s = self.soft(b, 0.02)?;
                smooth(level.clone() - s.clone(), level + s, shape)
            }

            // ── scattering ──
            "scatter" => {
                let count = self.whole(b, "count", Def::RandInt(20, 80), 0, 100_000)?;
                let size = match b.get("size") {
                    Some(a) => self.scalar(&a.val, "`size`")?,
                    None => 0.012,
                };
                let events = (0..count)
                    .map(|_| Event {
                        x: self.rng.random(),
                        y: self.rng.random(),
                        t: self.rng.random(),
                        intensity: 0.5 + 0.5 * self.rng.random::<f64>(),
                        size: (size * (0.5 + self.rng.random::<f64>())).abs().max(1e-4),
                        pitch: None,
                        length: None,
                    })
                    .collect();
                Field::Pattern(Arc::new(events))
            }
            // ── your own content ──
            "text" => {
                let (words, span) = self.string(b, "words", None)?;
                if words.trim().is_empty() {
                    return Err(Error::new("there is no text to draw", span));
                }
                let size = self.p(b, "size", Def::Rand(0.06, 0.15))?;
                let (font_name, font_span) = self.string(b, "font", Some("sans"))?;
                let (key, font) = crate::raster::font(&font_name, &self.module.dir).map_err(|e| {
                    let names: Vec<_> = crate::raster::FONTS.iter().map(|(n, _)| *n).collect();
                    Error::new(
                        format!(
                            "cannot use font `{font_name}`: {e} — use a built-in font ({}) or the path to a .ttf/.otf file",
                            names.join(", ")
                        ),
                        font_span,
                    )
                })?;
                // Sharp enough for the size it will be shown at.
                let line_px = size.constant().map(|s| (s * 2048.0).clamp(48.0, 1024.0)).unwrap_or(512.0) as u32;
                let img = crate::raster::rasterize_text(&key, &font, &words, line_px);
                let width = size / img.aspect();
                // `align`: where x is: the left end of the line, its centre, or its right end
                let (align, align_span) = self.string(b, "align", Some("center"))?;
                let shift = match align.as_str() {
                    "left" => 0.5,
                    "center" | "centre" => 0.0,
                    "right" => -0.5,
                    _ => return Err(Error::new("`align` is \"left\", \"center\" or \"right\"", align_span)),
                };
                self.raster_shifted(b, img, width, shift)?
            }
            "picture" => {
                let (file, span) = self.string(b, "file", None)?;
                let (channel, ch_span) = self.string(b, "channel", Some("luminance"))?;
                if !crate::raster::CHANNELS.contains(&channel.as_str()) {
                    return Err(Error::new(
                        format!("`channel` must be one of: {}", crate::raster::CHANNELS.join(", ")),
                        ch_span,
                    ));
                }
                let path = self.module.dir.join(&file);
                let img = crate::raster::load_picture(&path, &channel)
                    .map_err(|e| Error::new(format!("cannot open picture `{file}`: {e}"), span))?;
                let width = self.p(b, "size", Def::Fixed(1.0))?;
                self.raster(b, img, width)?
            }
            other => unreachable!("primitive `{other}` has parameters but no body"),
        };
        Ok(Val::Field(f))
    }
}

#[cfg(test)]
mod align_tests {
    /// `align: "left"` puts the start of the line at x; "right" its end.
    #[test]
    fn text_aligns() {
        let dir = std::env::temp_dir().join(format!("alea-align-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ink = |align: &str| -> (f64, f64) {
            let path = dir.join(format!("{align}.alea"));
            std::fs::write(&path, format!("work T {{\n  render image(text(\"MMMM\", 0.5, 0.5, 0.1, 0, \"sans\", \"{align}\"), width: 200)\n}}\n")).unwrap();
            let m = crate::library::load(path.to_str().unwrap()).unwrap();
            let ed = crate::eval::resolve(&m, 1).unwrap();
            let f = ed.renders[0].required_field(0, "f").unwrap();
            // the leftmost and rightmost x with ink, along the line
            let xs: Vec<f64> = (0..400).map(|k| k as f64 / 399.0).filter(|x| f.eval([*x, 0.5, 0.0], 1) > 0.5).collect();
            (xs[0], xs[xs.len() - 1])
        };
        let (l0, l1) = ink("left");
        let (c0, c1) = ink("center");
        let (r0, r1) = ink("right");
        assert!((l0 - 0.5).abs() < 0.02, "left: starts at x ({l0})");
        assert!(((c0 + c1) / 2.0 - 0.5).abs() < 0.02, "center: around x");
        assert!((r1 - 0.5).abs() < 0.02, "right: ends at x ({r1})");
        assert!(((l1 - l0) - (r1 - r0)).abs() < 0.01, "the same line, moved");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
