//! Inputs: what comes through a pipe (`cat photo.png | alea work.alea`), or
//! with `--in name=file`, or from a file named in the work (`input(or: "a.png")`).
//!
//! An input is recognised by its first bytes: an image (PNG, JPEG, WebP), a
//! sound (WAV), or text. The work sees it through `input()`:
//! - an image as fields (`.luma`, `.red`, `.hue`…), a painting (`.painting`)
//!   and a palette (`colours(photo, 5)`);
//! - a sound as fields of time (`.level`, `.bass`, `band(song, lo, hi)`…),
//!   a pattern of its beats (`.beats`), and itself, as a soundtrack;
//! - text as words, lines and rows.
//!
//! Nothing here draws from the edition's randomness: an input changes what a
//! work shows, never the order of its draws.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use crate::raster::Raster;

#[derive(Debug)]
pub struct Input {
    /// "" for the default input (the pipe), otherwise its name or file.
    pub name: String,
    /// A fingerprint of the bytes (FNV-1a), shown with the edition.
    pub hash: u64,
    pub kind: Kind,
}

#[derive(Debug)]
pub enum Kind {
    Image(Image),
    Sound(Clip),
    Text(String),
}

impl Input {
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            Kind::Image(_) => "an image",
            Kind::Sound(_) => "a sound",
            Kind::Text(_) => "a text",
        }
    }
}

// ───────────────────────────── the registry ─────────────────────────────

static INPUTS: LazyLock<Mutex<HashMap<String, Arc<Input>>>> = LazyLock::new(Default::default);
static FILES: LazyLock<Mutex<HashMap<PathBuf, Arc<Input>>>> = LazyLock::new(Default::default);
static USED: LazyLock<Mutex<Vec<Arc<Input>>>> = LazyLock::new(Default::default);
/// How to read the pipe, set by the command line. Read at most once, and
/// only when a work asks for its input.
static STDIN: OnceLock<Box<dyn Fn() -> Option<Vec<u8>> + Send + Sync>> = OnceLock::new();
static STDIN_READ: Mutex<bool> = Mutex::new(false);

pub fn set_stdin_reader(f: Box<dyn Fn() -> Option<Vec<u8>> + Send + Sync>) {
    let _ = STDIN.set(f);
}

/// Hands an input over under a name ("" for the default one).
pub fn put(name: &str, bytes: Vec<u8>) -> Result<Arc<Input>, String> {
    let i = Arc::new(decode(name, &bytes)?);
    INPUTS.lock().unwrap().insert(name.to_string(), i.clone());
    Ok(i)
}

pub fn clear() {
    INPUTS.lock().unwrap().clear();
    USED.lock().unwrap().clear();
}

/// The input of that name; the default one is read from the pipe the first time.
pub fn get(name: &str) -> Result<Option<Arc<Input>>, String> {
    if let Some(i) = INPUTS.lock().unwrap().get(name) {
        return Ok(Some(i.clone()));
    }
    if name.is_empty() {
        let mut read = STDIN_READ.lock().unwrap();
        if !*read {
            *read = true;
            if let Some(bytes) = STDIN.get().and_then(|f| f()) {
                if !bytes.is_empty() {
                    return put("", bytes).map(Some).map_err(|e| format!("the input in the pipe: {e}"));
                }
            }
        }
    }
    Ok(None)
}

/// An input from a file (the `or:` of `input`), read once.
pub fn from_file(path: &Path, shown: &str) -> Result<Arc<Input>, String> {
    if let Some(i) = FILES.lock().unwrap().get(path) {
        return Ok(i.clone());
    }
    let bytes = crate::vfs::read(path).map_err(|e| e.to_string())?;
    let i = Arc::new(decode(shown, &bytes)?);
    FILES.lock().unwrap().insert(path.to_path_buf(), i.clone());
    Ok(i)
}

pub fn mark_used(i: &Arc<Input>) {
    let mut u = USED.lock().unwrap();
    if !u.iter().any(|x| Arc::ptr_eq(x, i)) {
        u.push(i.clone());
    }
}

pub fn reset_used() {
    USED.lock().unwrap().clear();
}

pub fn used() -> Vec<Arc<Input>> {
    USED.lock().unwrap().clone()
}

/// The canvas an image input suggests (its own size), and the length a sound
/// input suggests (its duration), from the inputs used so far.
pub fn hints() -> (Option<(usize, usize)>, Option<f64>) {
    let u = USED.lock().unwrap();
    let canvas = u.iter().find_map(|i| match &i.kind {
        Kind::Image(img) => Some((img.w, img.h)),
        _ => None,
    });
    let length = u
        .iter()
        .filter_map(|i| match &i.kind {
            Kind::Sound(c) => Some(c.duration()),
            _ => None,
        })
        .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.max(d))));
    (canvas, length)
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Recognises an input by its first bytes.
pub fn decode(name: &str, bytes: &[u8]) -> Result<Input, String> {
    let hash = fnv(bytes);
    let riff = |tag: &[u8]| bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == tag;
    let kind = if bytes.starts_with(b"\x89PNG") || bytes.starts_with(&[0xFF, 0xD8, 0xFF]) || riff(b"WEBP") {
        Kind::Image(Image::decode(bytes)?)
    } else if riff(b"WAVE") {
        Kind::Sound(Clip::decode_wav(bytes)?)
    } else if bytes.starts_with(b"fLaC") || bytes.starts_with(b"ID3") || bytes.starts_with(b"OggS") || bytes.starts_with(&[0xFF, 0xFB]) {
        return Err("this sound format isn't read directly: convert it to WAV, for example \
                    `ffmpeg -i song.mp3 -f wav - | alea work.alea`"
            .into());
    } else {
        match std::str::from_utf8(bytes) {
            Ok(s) => Kind::Text(s.to_string()),
            Err(_) => return Err("not an image (PNG, JPEG, WebP), a sound (WAV) or a text".into()),
        }
    };
    Ok(Input { name: name.to_string(), hash, kind })
}

// ───────────────────────────── images ─────────────────────────────

const MAX_SIDE: u32 = 4096;

#[derive(Debug)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    /// Straight RGBA, from 0 to 1.
    pub px: Vec<[f32; 4]>,
    channels: Mutex<HashMap<String, Arc<Raster>>>,
}

pub const CHANNELS: &[&str] = &["luma", "red", "green", "blue", "alpha", "hue", "saturation", "lightness"];

impl Image {
    fn decode(bytes: &[u8]) -> Result<Image, String> {
        let mut img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
        if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
            img = img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle);
        }
        let rgba = img.to_rgba8();
        let px = rgba.pixels().map(|p| p.0.map(|c| c as f32 / 255.0)).collect();
        Ok(Image { w: rgba.width() as usize, h: rgba.height() as usize, px, channels: Mutex::new(HashMap::new()) })
    }

    /// Height over width.
    pub fn aspect(&self) -> f64 {
        self.h as f64 / self.w as f64
    }

    /// The width that makes the picture cover alea's canvas of its own
    /// proportions exactly: its shorter side spans 0 to 1.
    pub fn cover_width(&self) -> f64 {
        (1.0 / self.aspect()).max(1.0)
    }

    /// One channel as values from 0 to 1 (transparent pixels count as 0).
    pub fn channel(&self, name: &str) -> Arc<Raster> {
        if let Some(r) = self.channels.lock().unwrap().get(name) {
            return r.clone();
        }
        let data = self
            .px
            .iter()
            .map(|&[r, g, b, a]| {
                let (max, min) = (r.max(g).max(b), r.min(g).min(b));
                match name {
                    "red" => r * a,
                    "green" => g * a,
                    "blue" => b * a,
                    "alpha" => a,
                    "lightness" => (max + min) * 0.5 * a,
                    "saturation" => {
                        if max <= 0.0 { 0.0 } else { (max - min) / max * a }
                    }
                    "hue" => {
                        let d = max - min;
                        if d <= 1e-6 {
                            0.0
                        } else {
                            let h = if max == r {
                                ((g - b) / d).rem_euclid(6.0)
                            } else if max == g {
                                (b - r) / d + 2.0
                            } else {
                                (r - g) / d + 4.0
                            };
                            h / 6.0
                        }
                    }
                    _ => (0.2126 * r + 0.7152 * g + 0.0722 * b) * a,
                }
            })
            .collect();
        let r = Arc::new(Raster { w: self.w, h: self.h, data });
        self.channels.lock().unwrap().insert(name.to_string(), r.clone());
        r
    }

    /// Bilinear RGBA at (u, v) ∈ [0, 1]²; transparent outside.
    pub fn sample(&self, u: f64, v: f64) -> [f32; 4] {
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return [0.0; 4];
        }
        let (x, y) = (u * self.w as f64 - 0.5, v * self.h as f64 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
        let at = |i: f64, j: f64| {
            let i = i.clamp(0.0, self.w as f64 - 1.0) as usize;
            let j = j.clamp(0.0, self.h as f64 - 1.0) as usize;
            self.px[j * self.w + i]
        };
        let (a, b, c, d) = (at(x0, y0), at(x0 + 1.0, y0), at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0));
        let mut out = [0.0f32; 4];
        for k in 0..4 {
            let top = a[k] * (1.0 - fx) + b[k] * fx;
            let bottom = c[k] * (1.0 - fx) + d[k] * fx;
            out[k] = top * (1.0 - fy) + bottom * fy;
        }
        out
    }

    /// The `n` colours that best sum the picture up (k-means on a sample of
    /// its pixels), from the darkest to the lightest. Always the same for the
    /// same picture.
    pub fn palette(&self, n: usize) -> Vec<[u8; 3]> {
        // at most about 16 000 pixels, evenly spread
        let step = ((self.w * self.h) as f64 / 16384.0).sqrt().max(1.0);
        let mut pts: Vec<[f64; 3]> = Vec::new();
        let mut y = 0.0;
        while (y as usize) < self.h {
            let mut x = 0.0;
            while (x as usize) < self.w {
                let p = self.px[y as usize * self.w + x as usize];
                if p[3] > 0.5 {
                    pts.push([p[0] as f64, p[1] as f64, p[2] as f64]);
                }
                x += step;
            }
            y += step;
        }
        if pts.is_empty() {
            return vec![[0, 0, 0]; n];
        }
        let d2 = |a: &[f64; 3], b: &[f64; 3]| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>();
        // k-means++ seeding, with a fixed generator: the farthest colours first
        let mut rng = 0x5eed_u64;
        let mut unit = || {
            rng = crate::eval::hash(&[rng]);
            (rng >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut centres = vec![pts[pts.len() / 2]];
        let mut near: Vec<f64> = pts.iter().map(|p| d2(p, &centres[0])).collect();
        while centres.len() < n {
            let total: f64 = near.iter().sum();
            if total <= 1e-12 {
                centres.push(centres[centres.len() - 1]);
                continue;
            }
            let mut u = unit() * total;
            let mut k = pts.len() - 1;
            for (j, d) in near.iter().enumerate() {
                if u < *d {
                    k = j;
                    break;
                }
                u -= d;
            }
            centres.push(pts[k]);
            for (j, p) in pts.iter().enumerate() {
                near[j] = near[j].min(d2(p, &centres[centres.len() - 1]));
            }
        }
        for _ in 0..16 {
            let mut sums = vec![[0.0f64; 4]; n];
            for p in &pts {
                let k = (0..n).min_by(|&a, &b| d2(p, &centres[a]).total_cmp(&d2(p, &centres[b]))).unwrap();
                for c in 0..3 {
                    sums[k][c] += p[c];
                }
                sums[k][3] += 1.0;
            }
            for k in 0..n {
                if sums[k][3] > 0.0 {
                    centres[k] = [sums[k][0] / sums[k][3], sums[k][1] / sums[k][3], sums[k][2] / sums[k][3]];
                }
            }
        }
        let lum = |c: &[f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        centres.sort_by(|a, b| lum(a).total_cmp(&lum(b)));
        centres.iter().map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)).collect()
    }
}

// ───────────────────────────── sounds ─────────────────────────────

/// Curves of a sound are sampled this many times per second.
pub const CURVE_RATE: f64 = 200.0;

#[derive(Debug)]
pub struct Clip {
    pub rate: u32,
    /// One or two channels, from −1 to 1.
    pub channels: Vec<Vec<f32>>,
    curves: Mutex<HashMap<String, Arc<Vec<f32>>>>,
    beats: OnceLock<Vec<(f64, f64)>>,
}

impl Clip {
    pub fn duration(&self) -> f64 {
        self.channels[0].len() as f64 / self.rate as f64
    }

    /// A WAV file: PCM 8, 16, 24 or 32 bits, or 32/64-bit float. A size of
    /// "unknown" (what a program writing into a pipe puts) means "until the end".
    fn decode_wav(b: &[u8]) -> Result<Clip, String> {
        let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let mut pos = 12;
        let mut fmt: Option<(u16, u16, u32, u16)> = None;
        let mut data: Option<&[u8]> = None;
        while pos + 8 <= b.len() {
            let id = &b[pos..pos + 4];
            let size = u32le(pos + 4) as usize;
            let body = pos + 8;
            let end = if size == 0xFFFF_FFFF || body + size > b.len() { b.len() } else { body + size };
            if id == b"fmt " && end - body >= 16 {
                let mut format = u16le(body);
                if format == 0xFFFE && end - body >= 26 {
                    format = u16le(body + 24); // WAVE_FORMAT_EXTENSIBLE: the real format is in the sub-format
                }
                fmt = Some((format, u16le(body + 2), u32le(body + 4), u16le(body + 14)));
            } else if id == b"data" {
                data = Some(&b[body..end]);
                break;
            }
            pos = end + (end & 1);
        }
        let (format, nch, rate, bits) = fmt.ok_or("a WAV file without its format")?;
        let data = data.ok_or("a WAV file without sound data")?;
        if nch == 0 || rate == 0 {
            return Err("a WAV file with no channels".into());
        }
        let width = (bits as usize).div_ceil(8);
        let frame = width * nch as usize;
        let frames = data.len() / frame.max(1);
        if frames == 0 {
            return Err("a WAV file with no sound in it".into());
        }
        let sample = |o: usize| -> f32 {
            let s = &data[o..o + width];
            match (format, width) {
                (1, 1) => (s[0] as f32 - 128.0) / 128.0,
                (1, 2) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
                (1, 3) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.0,
                (1, 4) => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
                (3, 4) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                (3, 8) => f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]) as f32,
                _ => f32::NAN,
            }
        };
        if sample(0).is_nan() {
            return Err(format!("a WAV encoding this program doesn't read (format {format}, {bits} bits): use 16-bit PCM"));
        }
        // keep at most two channels (the first two), at most 10 minutes
        let keep = (nch as usize).min(2);
        let frames = frames.min(rate as usize * 600);
        let channels = (0..keep)
            .map(|c| (0..frames).map(|k| sample(k * frame + c * width).clamp(-1.0, 1.0)).collect())
            .collect();
        Ok(Clip { rate, channels, curves: Mutex::new(HashMap::new()), beats: OnceLock::new() })
    }

    fn mono(&self) -> Vec<f32> {
        match self.channels.as_slice() {
            [m] => m.clone(),
            [l, r, ..] => l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect(),
            [] => Vec::new(),
        }
    }

    /// The loudness of `signal` over time, `CURVE_RATE` values per second.
    fn envelope(&self, signal: &[f32]) -> Vec<f32> {
        let hop = (self.rate as f64 / CURVE_RATE).max(1.0);
        let n = (signal.len() as f64 / hop).ceil() as usize;
        let win = (2.0 * hop) as usize;
        let mut raw = Vec::with_capacity(n);
        for k in 0..n {
            let c = (k as f64 * hop) as usize;
            let (a, b) = (c.saturating_sub(win / 2), (c + win / 2).min(signal.len()));
            let e = signal[a..b].iter().map(|x| x * x).sum::<f32>() / (b - a).max(1) as f32;
            raw.push(e.sqrt());
        }
        raw
    }

    /// Smooths an envelope (quick to rise, slower to fall) and scales it so
    /// that its loud moments reach 1.
    fn shape(raw: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(raw.len());
        let mut e = 0.0f32;
        for &v in raw {
            let k = if v > e { 0.5 } else { 0.06 };
            e += (v - e) * k;
            out.push(e);
        }
        let mut sorted = out.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let top = sorted.get(((sorted.len() as f64) * 0.99) as usize).copied().unwrap_or(1.0).max(1e-6);
        out.iter().map(|v| (v / top).min(1.0)).collect()
    }

    fn band_signal(&self, lo: f64, hi: f64) -> Vec<f32> {
        let nyq = self.rate as f64 / 2.0;
        let mut s = self.mono();
        if lo > 1.0 {
            let lo = lo.min(nyq * 0.95);
            for _ in 0..2 {
                Biquad::highpass(lo, self.rate as f64).run(&mut s);
            }
        }
        if hi < nyq * 0.95 {
            for _ in 0..2 {
                Biquad::lowpass(hi, self.rate as f64).run(&mut s);
            }
        }
        s
    }

    /// `level`, `bass`, `mid`, `treble`, a band `lo-hi`, or `pulse`: values
    /// from 0 to 1 along the sound.
    pub fn curve(&self, name: &str, lo: f64, hi: f64) -> Arc<Vec<f32>> {
        let key = format!("{name}:{lo}:{hi}");
        if let Some(c) = self.curves.lock().unwrap().get(&key) {
            return c.clone();
        }
        let c = Arc::new(match name {
            "level" => Self::shape(&self.envelope(&self.mono())),
            "pulse" => {
                let n = (self.duration() * CURVE_RATE).ceil() as usize;
                let mut out = vec![0.0f32; n.max(1)];
                for &(time, strength) in self.beats() {
                    let start = (time * CURVE_RATE) as usize;
                    for k in start..out.len() {
                        let dt = (k - start) as f64 / CURVE_RATE;
                        let v = (strength * (-dt / 0.18).exp()) as f32;
                        if v < 0.002 {
                            break;
                        }
                        out[k] = out[k].max(v);
                    }
                }
                out
            }
            _ => Self::shape(&self.envelope(&self.band_signal(lo, hi))),
        });
        self.curves.lock().unwrap().insert(key, c.clone());
        c
    }

    /// The beats: moments where the sound suddenly grows (onsets), with how
    /// strong each is (0 to 1).
    pub fn beats(&self) -> &[(f64, f64)] {
        self.beats.get_or_init(|| {
            // the rise of the loudness in three bands, on a log scale
            let bands = [(20.0, 200.0), (200.0, 2000.0), (2000.0, 16000.0)];
            let envs: Vec<Vec<f32>> = bands.iter().map(|&(lo, hi)| self.envelope(&self.band_signal(lo, hi))).collect();
            let n = envs[0].len();
            let mut flux = vec![0.0f64; n];
            for e in &envs {
                let peak = e.iter().fold(1e-9f32, |m, v| m.max(*v)) as f64;
                // before the start, silence: a sound that starts loud starts with a beat
                flux[0] += ((e[0] as f64 / peak + 1e-3).ln() - 1e-3f64.ln()).max(0.0);
                for k in 1..n {
                    let (a, b) = ((e[k - 1] as f64 / peak + 1e-3).ln(), (e[k] as f64 / peak + 1e-3).ln());
                    flux[k] += (b - a).max(0.0);
                }
            }
            // a peak higher than its neighbourhood (half a second around)
            let w = (0.25 * CURVE_RATE) as usize;
            let local = (0.05 * CURVE_RATE) as usize;
            let gap = 0.1 * CURVE_RATE;
            let mut found: Vec<(f64, f64)> = Vec::new();
            let mut last = -1e9f64;
            for k in 0..n.saturating_sub(1) {
                let (a, b) = (k.saturating_sub(w), (k + w).min(n));
                let mean = flux[a..b].iter().sum::<f64>() / (b - a) as f64;
                let var = flux[a..b].iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (b - a) as f64;
                let (la, lb) = (k.saturating_sub(local), (k + local + 1).min(n));
                let is_max = flux[la..lb].iter().all(|v| *v <= flux[k]);
                if is_max && flux[k] > mean + 1.2 * var.sqrt() && flux[k] > 0.15 && (k as f64 - last) >= gap {
                    found.push((k as f64 / CURVE_RATE, flux[k]));
                    last = k as f64;
                }
            }
            let mut strengths: Vec<f64> = found.iter().map(|b| b.1).collect();
            strengths.sort_by(|a, b| a.total_cmp(b));
            let top = strengths.get((strengths.len() as f64 * 0.9) as usize).copied().unwrap_or(1.0).max(1e-9);
            // the faint ones (a fifth of a typical beat) are not beats
            found.iter().filter(|b| b.1 / top >= 0.2).map(|&(t, s)| (t, (s / top).min(1.0))).collect()
        })
    }

    /// The sound resampled to `rate`, as two channels, `n` samples long
    /// (silence after its end).
    pub fn stereo(&self, rate: f64, n: usize) -> [Vec<f64>; 2] {
        let ratio = self.rate as f64 / rate;
        let at = |ch: &[f32], k: usize| -> f64 {
            let x = k as f64 * ratio;
            let i = x as usize;
            if i + 1 >= ch.len() {
                return if i < ch.len() { ch[i] as f64 } else { 0.0 };
            }
            let f = x - i as f64;
            ch[i] as f64 * (1.0 - f) + ch[i + 1] as f64 * f
        };
        let l = &self.channels[0];
        let r = self.channels.get(1).unwrap_or(l);
        [(0..n).map(|k| at(l, k)).collect(), (0..n).map(|k| at(r, k)).collect()]
    }
}

/// A two-pole filter (RBJ cookbook), Butterworth Q.
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
}

impl Biquad {
    fn new(f: f64, rate: f64, high: bool) -> Biquad {
        let w = std::f64::consts::TAU * f / rate;
        let (s, c) = (w.sin(), w.cos());
        let alpha = s / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let b = if high { [(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0] } else { [(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0] };
        Biquad { b: b.map(|v| v / a0), a: [-2.0 * c / a0, (1.0 - alpha) / a0] }
    }
    fn highpass(f: f64, rate: f64) -> Biquad {
        Biquad::new(f, rate, true)
    }
    fn lowpass(f: f64, rate: f64) -> Biquad {
        Biquad::new(f, rate, false)
    }
    fn run(&self, s: &mut [f32]) {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0, 0.0, 0.0);
        for v in s.iter_mut() {
            let x = *v as f64;
            let y = self.b[0] * x + self.b[1] * x1 + self.b[2] * x2 - self.a[0] * y1 - self.a[1] * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            *v = y as f32;
        }
    }
}

// ───────────────────────────── text ─────────────────────────────

/// Each line split on commas, semicolons, tabs or spaces; a cell that reads
/// as a number is a number, otherwise it stays text. Empty lines are skipped.
pub fn rows(text: &str) -> Vec<Vec<Result<f64, String>>> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let sep: &[char] = if l.contains([',', ';', '\t']) { &[',', ';', '\t'] } else { &[' '] };
            l.split(sep)
                .map(|c| c.trim())
                .filter(|c| !c.is_empty() || sep[0] != ' ')
                .map(|c| c.parse::<f64>().map_err(|_| c.trim_matches('"').to_string()))
                .collect()
        })
        .collect()
}

