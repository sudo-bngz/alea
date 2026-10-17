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
    /// `at(painting, time)`: the painting at another moment.
    Retime { inner: Arc<Paint>, time: Field, seed: u64 },
    /// `sequence([…], fade)`: one painting after another, crossfading.
    Sequence { items: Vec<Arc<Paint>>, fade: f64, time: Field, seed: u64 },
    /// A filter on the pixels.
    Filter { inner: Arc<Paint>, kind: FilterKind, seed: u64 },
    /// An image input in its own colours, centred, `width` wide.
    Picture { input: Arc<crate::input::Input>, width: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterKind {
    /// A Gaussian blur; its spread as a fraction of the canvas.
    Blur(f64),
    /// Square blocks of that size.
    Pixelate(f64),
    /// Light bleeding out of the bright parts.
    Bloom(f64),
    /// Film grain, new at every frame.
    Film(f64),
    /// Darker towards the corners.
    Vignette(f64),
    /// Red and blue drifting apart towards the edges, like a cheap lens.
    Chroma(f64),
}

/// How much each item of a sequence shows at moment `t` (0 to 1): item i
/// holds the i-th share of the time, crossfading over `fade` at each change.
pub fn sequence_weights(n: usize, fade: f64, t: f64) -> Vec<f64> {
    let ramp = |a: f64, b: f64, v: f64| if b <= a { if v >= a { 1.0 } else { 0.0 } } else { ((v - a) / (b - a)).clamp(0.0, 1.0) };
    (0..n)
        .map(|i| {
            let (start, end) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
            let rise = if i == 0 { 1.0 } else { ramp(start - fade / 2.0, start + fade / 2.0, t) };
            let fall = if i + 1 == n { 1.0 } else { 1.0 - ramp(end - fade / 2.0, end + fade / 2.0, t) };
            rise.min(fall)
        })
        .collect()
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
        Paint::Retime { inner, time, seed } => draw(inner, w, h, time.eval([0.5, 0.5, t], *seed)),
        Paint::Sequence { items, fade, time, seed } => {
            let at = time.eval([0.5, 0.5, t], *seed);
            let weights = sequence_weights(items.len(), *fade, at);
            let mut out = vec![[0.0f32; 4]; w * h];
            for (item, wt) in items.iter().zip(weights) {
                if wt <= 0.0 {
                    continue;
                }
                let px = draw(item, w, h, t);
                for (o, p) in out.iter_mut().zip(px.iter()) {
                    for c in 0..4 {
                        o[c] += p[c] * wt as f32;
                    }
                }
            }
            out
        }
        Paint::Filter { inner, kind, seed } => filter(draw(inner, w, h, t), w, h, t, *kind, *seed),
        Paint::Picture { input, width } => {
            let crate::input::Kind::Image(img) = &input.kind else { return vec![[0.0; 4]; w * h] };
            let s = w.min(h) as f64;
            let (ox, oy) = (0.5 - w as f64 / (2.0 * s), 0.5 - h as f64 / (2.0 * s));
            let height = width * img.aspect();
            let mut out = Vec::with_capacity(w * h);
            for j in 0..h {
                let y = oy + (j as f64 + 0.5) / s;
                for i in 0..w {
                    let x = ox + (i as f64 + 0.5) / s;
                    out.push(img.sample((x - 0.5) / width + 0.5, (y - 0.5) / height + 0.5));
                }
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

// ───────────────────────────── filters ─────────────────────────────

fn premultiply(px: &mut Pixels) {
    for p in px.iter_mut() {
        for c in 0..3 {
            p[c] *= p[3];
        }
    }
}

fn unpremultiply(px: &mut Pixels) {
    for p in px.iter_mut() {
        if p[3] > 1e-6 {
            for c in 0..3 {
                p[c] = (p[c] / p[3]).min(1.0);
            }
        }
    }
}

/// A separable Gaussian blur of `sigma` pixels (on premultiplied pixels).
fn gaussian(px: &Pixels, w: usize, h: usize, sigma: f64) -> Pixels {
    if sigma < 0.3 {
        return px.clone();
    }
    let r = ((sigma * 3.0).ceil() as usize).min(256);
    let kernel: Vec<f32> = {
        let k: Vec<f64> = (0..=2 * r).map(|i| (-((i as f64 - r as f64) / sigma).powi(2) / 2.0).exp()).collect();
        let sum: f64 = k.iter().sum();
        k.iter().map(|v| (v / sum) as f32).collect()
    };
    let pass = |src: &Pixels, horizontal: bool| -> Pixels {
        let mut out = vec![[0.0f32; 4]; w * h];
        for j in 0..h {
            for i in 0..w {
                let mut acc = [0.0f32; 4];
                for (k, wt) in kernel.iter().enumerate() {
                    let o = k as i64 - r as i64;
                    let (x, y) = if horizontal {
                        ((i as i64 + o).clamp(0, w as i64 - 1) as usize, j)
                    } else {
                        (i, (j as i64 + o).clamp(0, h as i64 - 1) as usize)
                    };
                    let p = src[y * w + x];
                    for c in 0..4 {
                        acc[c] += p[c] * wt;
                    }
                }
                out[j * w + i] = acc;
            }
        }
        out
    };
    pass(&pass(px, true), false)
}

fn filter(mut px: Pixels, w: usize, h: usize, t: f64, kind: FilterKind, seed: u64) -> Pixels {
    let s = w.min(h) as f64;
    match kind {
        FilterKind::Blur(r) => {
            premultiply(&mut px);
            let mut out = gaussian(&px, w, h, r * s);
            unpremultiply(&mut out);
            out
        }
        FilterKind::Pixelate(size) => {
            let b = ((size * s).round() as usize).max(1);
            premultiply(&mut px);
            let mut out = px.clone();
            for by in (0..h).step_by(b) {
                for bx in (0..w).step_by(b) {
                    let (x1, y1) = ((bx + b).min(w), (by + b).min(h));
                    let mut acc = [0.0f32; 4];
                    for y in by..y1 {
                        for x in bx..x1 {
                            for c in 0..4 {
                                acc[c] += px[y * w + x][c];
                            }
                        }
                    }
                    let n = ((x1 - bx) * (y1 - by)) as f32;
                    for y in by..y1 {
                        for x in bx..x1 {
                            out[y * w + x] = acc.map(|v| v / n);
                        }
                    }
                }
            }
            unpremultiply(&mut out);
            out
        }
        FilterKind::Bloom(amount) => {
            // what is bright, blurred twice (near and far), screened back on
            let mut bright: Pixels = px
                .iter()
                .map(|p| {
                    let lum = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
                    let k = ((lum - 0.55) / 0.45).clamp(0.0, 1.0) * p[3];
                    [p[0] * k, p[1] * k, p[2] * k, k]
                })
                .collect();
            let near = gaussian(&bright, w, h, 0.012 * s);
            bright = gaussian(&bright, w, h, 0.045 * s);
            for (k, p) in px.iter_mut().enumerate() {
                for c in 0..3 {
                    let g = ((near[k][c] + bright[k][c]) * amount as f32).min(1.0);
                    p[c] = 1.0 - (1.0 - p[c]) * (1.0 - g);
                }
            }
            px
        }
        FilterKind::Film(amount) => {
            let frame = t.to_bits();
            for (k, p) in px.iter_mut().enumerate() {
                let n = (crate::eval::hash(&[seed, k as u64, frame]) >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
                for c in 0..3 {
                    p[c] = (p[c] + (n * amount * 0.5) as f32).clamp(0.0, 1.0);
                }
            }
            px
        }
        FilterKind::Vignette(amount) => {
            let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
            let half = (cx * cx + cy * cy).sqrt();
            for j in 0..h {
                for i in 0..w {
                    let r = ((i as f64 + 0.5 - cx).powi(2) + (j as f64 + 0.5 - cy).powi(2)).sqrt() / half;
                    let e = ((r - 0.35) / 0.6).clamp(0.0, 1.0);
                    let k = (1.0 - amount * e * e * (3.0 - 2.0 * e)) as f32;
                    let p = &mut px[j * w + i];
                    for c in 0..3 {
                        p[c] *= k;
                    }
                }
            }
            px
        }
        FilterKind::Chroma(amount) => {
            let src = px.clone();
            let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
            let at = |x: f64, y: f64| {
                let (xi, yi) = (x.round().clamp(0.0, w as f64 - 1.0) as usize, y.round().clamp(0.0, h as f64 - 1.0) as usize);
                src[yi * w + xi]
            };
            for j in 0..h {
                for i in 0..w {
                    let (dx, dy) = (i as f64 - cx, j as f64 - cy);
                    let k = amount * 0.02;
                    px[j * w + i][0] = at(i as f64 + dx * k, j as f64 + dy * k)[0];
                    px[j * w + i][2] = at(i as f64 - dx * k, j as f64 - dy * k)[2];
                }
            }
            px
        }
    }
}
