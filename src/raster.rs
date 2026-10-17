//! Bitmaps inside fields: rasterized text and imported pictures.
//!
//! Both become a `Raster` (values 0–1) that a `Field::Raster` places on the
//! canvas, so text and pictures can be moved, rotated, warped, animated and
//! combined like any other shape. Loaded files and rasterized text are cached:
//! constraint retries and repeated calls don't load or draw them again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};

#[derive(Debug)]
pub struct Raster {
    pub w: usize,
    pub h: usize,
    pub data: Vec<f32>,
}

impl Raster {
    /// Height over width.
    pub fn aspect(&self) -> f64 {
        self.h as f64 / self.w as f64
    }

    /// A Gaussian blur of the bitmap, `sigma` pixels wide (edges repeated).
    pub fn blurred(&self, sigma: f64) -> Arc<Raster> {
        let (w, h) = (self.w, self.h);
        if sigma < 0.3 {
            return Arc::new(Raster { w, h, data: self.data.clone() });
        }
        let r = ((sigma * 3.0).ceil() as usize).min(512);
        let k: Vec<f64> = (0..=2 * r).map(|i| (-((i as f64 - r as f64) / sigma).powi(2) / 2.0).exp()).collect();
        let sum: f64 = k.iter().sum();
        let k: Vec<f32> = k.iter().map(|v| (v / sum) as f32).collect();
        let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
            let mut out = vec![0.0f32; w * h];
            for j in 0..h {
                for i in 0..w {
                    let mut acc = 0.0f32;
                    for (n, wt) in k.iter().enumerate() {
                        let o = n as i64 - r as i64;
                        let (x, y) = if horizontal {
                            ((i as i64 + o).clamp(0, w as i64 - 1) as usize, j)
                        } else {
                            (i, (j as i64 + o).clamp(0, h as i64 - 1) as usize)
                        };
                        acc += src[y * w + x] * wt;
                    }
                    out[j * w + i] = acc;
                }
            }
            out
        };
        let data = pass(&pass(&self.data, true), false);
        Arc::new(Raster { w, h, data })
    }

    /// Bilinear sample at (u, v) ∈ [0, 1]²; 0 outside.
    pub fn sample(&self, u: f64, v: f64) -> f64 {
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return 0.0;
        }
        let (x, y) = (u * self.w as f64 - 0.5, v * self.h as f64 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |i: f64, j: f64| -> f64 {
            if i < 0.0 || j < 0.0 || i >= self.w as f64 || j >= self.h as f64 {
                0.0
            } else {
                self.data[j as usize * self.w + i as usize] as f64
            }
        };
        let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

// ───────────────────────────── pictures ─────────────────────────────

pub const CHANNELS: &[&str] = &["luminance", "red", "green", "blue", "alpha"];
const MAX_SIDE: u32 = 4096;

static PICTURES: LazyLock<Mutex<HashMap<(PathBuf, String), Arc<Raster>>>> = LazyLock::new(Default::default);

pub fn load_picture(path: &Path, channel: &str) -> Result<Arc<Raster>, String> {
    let key = (path.to_path_buf(), channel.to_string());
    if let Some(r) = PICTURES.lock().unwrap().get(&key) {
        return Ok(r.clone());
    }
    let bytes = crate::vfs::read(path).map_err(|e| e.to_string())?;
    let mut img = match image::ImageFormat::from_path(path) {
        Ok(fmt) => image::load_from_memory_with_format(&bytes, fmt),
        Err(_) => image::load_from_memory(&bytes),
    }
    .map_err(|e| e.to_string())?;
    if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
        img = img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle);
    }
    let rgba = img.to_rgba8();
    let data = rgba
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0.map(|c| c as f32 / 255.0);
            match channel {
                "red" => r * a,
                "green" => g * a,
                "blue" => b * a,
                "alpha" => a,
                _ => (0.2126 * r + 0.7152 * g + 0.0722 * b) * a,
            }
        })
        .collect();
    let r = Arc::new(Raster { w: rgba.width() as usize, h: rgba.height() as usize, data });
    PICTURES.lock().unwrap().insert(key, r.clone());
    Ok(r)
}

// ───────────────────────────── text ─────────────────────────────

pub const FONTS: &[(&str, &[u8])] = &[
    ("sans", include_bytes!("../fonts/sans.ttf")),
    ("bold", include_bytes!("../fonts/bold.ttf")),
    ("serif", include_bytes!("../fonts/serif.ttf")),
    ("mono", include_bytes!("../fonts/mono.ttf")),
];

static LOADED_FONTS: LazyLock<Mutex<HashMap<String, FontArc>>> = LazyLock::new(Default::default);
static TEXTS: LazyLock<Mutex<HashMap<(String, String, u32), Arc<Raster>>>> = LazyLock::new(Default::default);

/// A built-in font by name, or a .ttf / .otf file (resolved against `dir`).
pub fn font(name: &str, dir: &Path) -> Result<(String, FontArc), String> {
    let key = match FONTS.iter().find(|(n, _)| *n == name) {
        Some(_) => name.to_string(),
        None => dir.join(name).display().to_string(),
    };
    if let Some(f) = LOADED_FONTS.lock().unwrap().get(&key) {
        return Ok((key, f.clone()));
    }
    let f = match FONTS.iter().find(|(n, _)| *n == name) {
        Some((_, bytes)) => FontArc::try_from_slice(bytes).map_err(|e| e.to_string())?,
        None => {
            let bytes = crate::vfs::read(Path::new(&key)).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => "no such file".to_string(),
                _ => e.to_string(),
            })?;
            FontArc::try_from_vec(bytes).map_err(|_| "not a font file this program can read".to_string())?
        }
    };
    LOADED_FONTS.lock().unwrap().insert(key.clone(), f.clone());
    Ok((key, f))
}

/// Draws one line of text; `line_px` is the height of the line in pixels.
pub fn rasterize_text(font_key: &str, font: &FontArc, text: &str, line_px: u32) -> Arc<Raster> {
    let key = (font_key.to_string(), text.to_string(), line_px);
    if let Some(r) = TEXTS.lock().unwrap().get(&key) {
        return r.clone();
    }
    // Scale so that ascent − descent (the line's height) is `line_px`.
    let unit = font.as_scaled(PxScale::from(1000.0));
    let scale = PxScale::from(1000.0 * line_px as f32 / (unit.ascent() - unit.descent()));
    let sf = font.as_scaled(scale);
    let ascent = sf.ascent();

    let mut caret = 0.0f32;
    let mut prev = None;
    let mut glyphs = Vec::new();
    for ch in text.chars() {
        let id = font.glyph_id(ch);
        if let Some(p) = prev {
            caret += sf.kern(p, id);
        }
        glyphs.push(id.with_scale_and_position(scale, point(caret, ascent)));
        caret += sf.h_advance(id);
        prev = Some(id);
    }

    let pad = 2usize;
    let w = caret.ceil().max(1.0) as usize + 2 * pad;
    let h = line_px as usize + 2 * pad;
    let mut data = vec![0.0f32; w * h];
    for g in glyphs {
        if let Some(o) = font.outline_glyph(g) {
            let bb = o.px_bounds();
            o.draw(|x, y, c| {
                let px = bb.min.x as i64 + x as i64 + pad as i64;
                let py = bb.min.y as i64 + y as i64 + pad as i64;
                if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                    let cell = &mut data[py as usize * w + px as usize];
                    *cell = cell.max(c);
                }
            });
        }
    }
    let r = Arc::new(Raster { w, h, data });
    TEXTS.lock().unwrap().insert(key, r.clone());
    r
}
