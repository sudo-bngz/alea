//! Paintings: fields in colour, laid out and stacked.
//!
//! A field is a grey world of numbers; a render gives it one palette. A
//! painting gives a field its own palette and its own range, so that several
//! works can share one canvas and each keep its colours:
//!
//! - `paint(field, palette)`: the field in colour (and transparent where its
//!   `alpha` is 0).
//! - `mosaic(paintings, columns)`: a grid, each cell showing a whole painting.
//! - `place(painting, x, y, width)`: one painting, anywhere, at any size.
//! - `overlay(paintings, blend, opacity, mask)`: stacked, with blend modes.
//!
//! Paintings are drawn as pixels: each layer is computed at the size it
//! occupies (with all of the engine's speed-ups), then composited.

use std::sync::{Arc, OnceLock};

use crate::eval::Field;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blend {
    Normal,
    Add,
    Screen,
    Multiply,
    Lighten,
    Darken,
    Difference,
    Overlay,
}

pub const BLENDS: &[&str] = &["normal", "add", "screen", "multiply", "lighten", "darken", "difference", "overlay"];

impl Blend {
    pub fn parse(s: &str) -> Option<Blend> {
        Some(match s {
            "normal" => Blend::Normal,
            "add" => Blend::Add,
            "screen" => Blend::Screen,
            "multiply" => Blend::Multiply,
            "lighten" => Blend::Lighten,
            "darken" => Blend::Darken,
            "difference" => Blend::Difference,
            "overlay" => Blend::Overlay,
            _ => return None,
        })
    }

    fn apply(self, a: f32, b: f32) -> f32 {
        match self {
            Blend::Normal => b,
            Blend::Add => (a + b).min(1.0),
            Blend::Screen => 1.0 - (1.0 - a) * (1.0 - b),
            Blend::Multiply => a * b,
            Blend::Lighten => a.max(b),
            Blend::Darken => a.min(b),
            Blend::Difference => (a - b).abs(),
            Blend::Overlay => {
                if a < 0.5 {
                    2.0 * a * b
                } else {
                    1.0 - 2.0 * (1.0 - a) * (1.0 - b)
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Paint {
    /// A field in colour. Its range is fixed (`range`) or measured once over
    /// the whole animation, so colours don't flicker.
    Field {
        field: Field,
        seed: u64,
        stops: crate::media::Palette,
        range: Option<(f64, f64)>,
        alpha: Option<Field>,
        bounds: OnceLock<(f64, f64)>,
    },
    /// A plain colour (a grey level, or a palette colour).
    Flat([f32; 3]),
    Mosaic { cells: Vec<Arc<Paint>>, columns: usize, gap: f64, background: Arc<Paint> },
    Place { inner: Arc<Paint>, x: f64, y: f64, width: f64, height: f64 },
    Overlay { layers: Vec<Arc<Paint>>, blend: Blend, opacity: f64, mask: Option<(Field, u64)> },
}

pub type Pixels = Vec<[f32; 4]>;

/// The painting drawn on a `w`×`h` frame at moment `t`, its unit square
/// centred as everywhere in alea. RGBA, from 0 to 1.
pub fn draw(p: &Paint, w: usize, h: usize, t: f64) -> Pixels {
    match p {
        Paint::Flat(c) => vec![[c[0], c[1], c[2], 1.0]; w * h],
        Paint::Field { field, seed, stops, range, alpha, bounds } => {
            let vals = crate::media::sample_frame(field, w, h, t, *seed);
            let b = range.unwrap_or_else(|| *bounds.get_or_init(|| crate::media::animation_bounds(field, 96, 96, *seed)));
            let a = alpha.as_ref().map(|f| crate::media::sample_frame(f, w, h, t, *seed));
            vals.iter()
                .enumerate()
                .map(|(k, v)| {
                    let c = stops.color(crate::media::normalize(*v, b));
                    let alpha = a.as_ref().map_or(1.0, |a| if a[k].is_finite() { a[k].clamp(0.0, 1.0) } else { 0.0 });
                    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, alpha as f32]
                })
                .collect()
        }
        Paint::Mosaic { cells, columns, gap, background } => {
            let mut out = draw(background, w, h, t);
            let n = cells.len().max(1);
            let cols = (*columns).clamp(1, n);
            let rows = n.div_ceil(cols);
            let g = (gap * w.min(h) as f64).round() as usize;
            let cw = w.saturating_sub(g * (cols + 1)) / cols;
            let ch = h.saturating_sub(g * (rows + 1)) / rows;
            if cw == 0 || ch == 0 {
                return out;
            }
            // what is left over is shared on both sides
            let (mx, my) = ((w - (cw * cols + g * (cols + 1))) / 2, (h - (ch * rows + g * (rows + 1))) / 2);
            for (k, cell) in cells.iter().enumerate() {
                let (c, r) = (k % cols, k / cols);
                let (x0, y0) = (mx + g + c * (cw + g), my + g + r * (ch + g));
                let px = draw(cell, cw, ch, t);
                paste(&mut out, w, h, &px, cw, ch, x0 as i64, y0 as i64);
            }
            out
        }
        Paint::Place { inner, x, y, width, height } => {
            let s = w.min(h) as f64;
            let (ox, oy) = (0.5 - w as f64 / (2.0 * s), 0.5 - h as f64 / (2.0 * s));
            let pw = (width * s).round().max(1.0) as usize;
            let ph = (height * s).round().max(1.0) as usize;
            let x0 = ((x - width / 2.0 - ox) * s).round() as i64;
            let y0 = ((y - height / 2.0 - oy) * s).round() as i64;
            let mut out = vec![[0.0f32; 4]; w * h];
            if pw <= 8192 && ph <= 8192 {
                let px = draw(inner, pw, ph, t);
                paste(&mut out, w, h, &px, pw, ph, x0, y0);
            }
            out
        }
        Paint::Overlay { layers, blend, opacity, mask } => {
            let m = mask.as_ref().map(|(f, seed)| crate::media::sample_frame(f, w, h, t, *seed));
            let mut out = draw(&layers[0], w, h, t);
            for layer in &layers[1..] {
                let top = draw(layer, w, h, t);
                for (k, (o, tp)) in out.iter_mut().zip(top.iter()).enumerate() {
                    let mk = m.as_ref().map_or(1.0, |m| if m[k].is_finite() { m[k].clamp(0.0, 1.0) as f32 } else { 0.0 });
                    let a = tp[3] * *opacity as f32 * mk;
                    for c in 0..3 {
                        // where the base is empty, the layer shows as it is
                        let mixed = blend.apply(o[c], tp[c]) * o[3] + tp[c] * (1.0 - o[3]);
                        o[c] += (mixed - o[c]) * a;
                    }
                    o[3] = o[3] + a * (1.0 - o[3]);
                }
            }
            out
        }
    }
}

/// `src` (sw×sh) laid over `dst` (w×h) at (x0, y0), through its alpha.
#[allow(clippy::too_many_arguments)]
fn paste(dst: &mut Pixels, w: usize, h: usize, src: &Pixels, sw: usize, sh: usize, x0: i64, y0: i64) {
    for j in 0..sh {
        let y = y0 + j as i64;
        if y < 0 || y >= h as i64 {
            continue;
        }
        for i in 0..sw {
            let x = x0 + i as i64;
            if x < 0 || x >= w as i64 {
                continue;
            }
            let s = src[j * sw + i];
            let d = &mut dst[y as usize * w + x as usize];
            let a = s[3];
            for c in 0..3 {
                d[c] += (s[c] - d[c]) * a;
            }
            d[3] = d[3] + a * (1.0 - d[3]);
        }
    }
}

/// RGB bytes of a painting, over black where it is transparent.
pub fn frame_rgb(p: &Paint, w: usize, h: usize, t: f64) -> Vec<u8> {
    let px = draw(p, w, h, t);
    let mut out = Vec::with_capacity(w * h * 3);
    for q in px {
        for c in 0..3 {
            out.push((q[c] * q[3]).clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8);
        }
    }
    out
}
