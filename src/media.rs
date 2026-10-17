//! What every medium computes, shared by the command line and the browser:
//! render parameters, colour mapping, frames, sound synthesis and sculpture
//! meshes. Writing files (PNG, WAV, MP4, STL) lives in `render.rs` (native) and
//! `web.rs` (browser). Shared convention: x, y ∈ [0, 1] (space), t ∈ [0, 1] (time).

use std::f64::consts::TAU;

use crate::error::{Error, Res};
use crate::eval::{Edition, Event, Field, Render, Val};

/// Maps `f` over 0..n, in parallel on native builds and sequentially in WebAssembly.
#[cfg(feature = "native")]
pub(crate) fn par_map<T: Send, F: Fn(usize) -> T + Sync + Send>(n: usize, f: F) -> Vec<T> {
    use rayon::prelude::*;
    (0..n).into_par_iter().map(f).collect()
}

#[cfg(not(feature = "native"))]
pub(crate) fn par_map<T, F: Fn(usize) -> T>(n: usize, f: F) -> Vec<T> {
    (0..n).map(f).collect()
}

// ───────────────────────────── arguments ─────────────────────────────

impl Render {
    pub(crate) fn check(&self, max_positional: usize, allowed: &[&str]) -> Res<()> {
        if let Some((_, span)) = self.positional.get(max_positional) {
            return Err(Error::new(
                format!("render `{}` takes at most {max_positional} unnamed argument(s)", self.kind),
                *span,
            ));
        }
        for (name, (_, span)) in &self.named {
            if !allowed.contains(&name.as_str()) {
                return Err(Error::new(
                    format!("unknown argument `{name}` for `{}` — allowed: {}", self.kind, allowed.join(", ")),
                    *span,
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn field(&self, k: usize, what: &str) -> Res<Option<&Field>> {
        match self.positional.get(k) {
            None => Ok(None),
            Some((Val::Field(f), _)) => Ok(Some(f)),
            Some((_, span)) => Err(Error::new(format!("{what} must be a field or a pattern"), *span)),
        }
    }

    /// What an image or a video shows: a field (through the render's palette)
    /// or a painting (in its own colours).
    pub fn visual(&self, what: &str) -> Res<Visual<'_>> {
        match self.positional.first() {
            Some((Val::Paint(p), span)) => {
                if self.named.contains_key("palette") {
                    return Err(Error::new("a painting has its own colours: give the palette to `paint(…)` instead", *span));
                }
                Ok(Visual::Paint(p))
            }
            _ => Ok(Visual::Field(self.required_field(0, what)?)),
        }
    }

    pub fn required_field(&self, k: usize, what: &str) -> Res<&Field> {
        self.field(k, what)?
            .ok_or_else(|| Error::new(format!("render `{}` expects {what}", self.kind), self.span))
    }

    pub(crate) fn number(&self, name: &str, default: f64) -> Res<f64> {
        match self.named.get(name) {
            None => Ok(default),
            Some((Val::Field(f), span)) => {
                f.constant().ok_or_else(|| Error::new(format!("`{name}` must be a fixed value"), *span))
            }
            Some((_, span)) => Err(Error::new(format!("`{name}` must be a number"), *span)),
        }
    }

    pub(crate) fn integer(&self, name: &str, default: f64, min: usize, max: usize) -> Res<usize> {
        let v = self.number(name, default)?;
        if !(v >= min as f64 && v <= max as f64) {
            let span = self.named.get(name).map(|(_, s)| *s).unwrap_or(self.span);
            return Err(Error::new(format!("`{name}` must be between {min} and {max}"), span));
        }
        Ok(v.round() as usize)
    }

    pub(crate) fn string(&self, name: &str, default: &str) -> Res<String> {
        match self.named.get(name) {
            None => Ok(default.to_string()),
            Some((Val::Str(s), _)) => Ok(s.clone()),
            Some((_, span)) => Err(Error::new(format!("`{name}` must be a string"), *span)),
        }
    }

    fn span_of(&self, name: &str) -> crate::error::Span {
        self.named.get(name).map(|(_, s)| *s).unwrap_or(self.span)
    }

    /// The optional second argument of `sound` and `video`: a pattern of events.
    pub fn pattern_arg(&self) -> Res<Option<&[Event]>> {
        match self.field(1, "the second argument")? {
            None => Ok(None),
            Some(p) => Ok(Some(p.pattern().map(|a| a.as_slice()).ok_or_else(|| {
                Error::new(format!("the second argument of `{}` must be a pattern", self.kind), self.positional[1].1)
            })?)),
        }
    }

    /// A sound input's duration if the work read one (at most 10 minutes), or `fallback`.
    fn default_length(&self, fallback: f64) -> f64 {
        self.length.map_or(fallback, |d| d.clamp(0.5, 600.0))
    }

    fn duration(&self, default: f64) -> Res<f64> {
        let d = self.number("duration", default)?;
        if !(0.5..=600.0).contains(&d) {
            return Err(Error::new("`duration` must be between 0.5 and 600 seconds", self.span_of("duration")));
        }
        Ok(d)
    }

    fn palette(&self) -> Res<Palette> {
        let bands = self.number("bands", 0.0)?;
        if !(bands == 0.0 || (bands >= 2.0 && bands <= 256.0 && bands.fract() == 0.0)) {
            return Err(Error::new("`bands` is a whole number of flat levels, from 2 to 256", self.span_of("bands")));
        }
        match self.named.get("palette") {
            None => Ok(Palette { bands: bands as usize, ..Palette::named("ink").expect("built in") }),
            Some((v, span)) => palette_from(v, *span, bands as usize),
        }
    }

    /// `range: [low, high]`: those values are the ends of the palette.
    fn range(&self) -> Res<Option<(f64, f64)>> {
        match self.named.get("range") {
            None => Ok(None),
            Some((Val::List(items), span)) => {
                let fixed: Vec<Option<f64>> = items.iter().map(|v| if let Val::Field(f) = v { f.constant() } else { None }).collect();
                match fixed.as_slice() {
                    [Some(lo), Some(hi)] if hi != lo => Ok(Some((*lo, *hi))),
                    _ => Err(Error::new("`range` is two different fixed numbers, [low, high]", *span)),
                }
            }
            Some((_, span)) => Err(Error::new("`range` is two fixed numbers, [low, high]", *span)),
        }
    }
}

// ───────────────────────────── render specs ─────────────────────────────
// Each render's parameters, read and checked once.

pub enum Visual<'a> {
    Field(&'a Field),
    Paint(&'a std::sync::Arc<crate::paint::Paint>),
}

impl Visual<'_> {
    /// One frame as RGB bytes. A field uses `stops` and the given bounds (or
    /// its own); a painting its own colours.
    pub fn rgb(&self, w: usize, h: usize, t: f64, seed: u64, stops: &Palette, fixed: Option<(f64, f64)>) -> Vec<u8> {
        match self {
            Visual::Field(f) => frame_rgb(f, w, h, t, seed, stops, fixed),
            Visual::Paint(p) => crate::paint::frame_rgb(p, w, h, t),
        }
    }
}

pub struct ImageSpec {
    pub w: usize,
    pub h: usize,
    pub at: f64,
    pub stops: Palette,
    pub range: Option<(f64, f64)>,
}

pub struct SoundSpec {
    pub duration: f64,
    pub fundamental: f64,
    pub voices: usize,
}

pub struct VideoSpec {
    pub w: usize,
    pub h: usize,
    pub fps: usize,
    pub duration: f64,
    pub stops: Palette,
    pub range: Option<(f64, f64)>,
    pub gif: bool,
    pub audio: bool,
    pub sound: SoundSpec,
}

pub struct SculptureSpec {
    pub n: usize,
    pub relief: f64,
    pub side: f64,
    pub base: f64,
    pub at: f64,
}

pub fn image_spec(r: &Render) -> Res<ImageSpec> {
    r.check(1, &["width", "height", "at", "palette", "bands", "range"])?;
    r.visual("the field to paint")?;
    // an image input: its own size by default, or its proportions
    let (w_default, ratio) = r.canvas.map_or((1024.0, 1.0), |(iw, ih)| (iw as f64, ih as f64 / iw as f64));
    let w = r.integer("width", w_default.max(16.0), 16, 8192)?;
    if matches!(r.visual("the field to paint")?, Visual::Paint(_)) && (r.named.contains_key("bands") || r.named.contains_key("range")) {
        return Err(Error::new("a painting has its own colours: give `bands` and `range` to `paint(…)` instead", r.span));
    }
    Ok(ImageSpec {
        w,
        h: r.integer("height", (w as f64 * ratio).round().max(16.0), 16, 8192)?,
        at: r.number("at", 0.0)?,
        stops: r.palette()?,
        range: r.range()?,
    })
}

fn sound_params(r: &Render, duration: f64) -> Res<SoundSpec> {
    Ok(SoundSpec {
        duration,
        fundamental: r.number("fundamental", 110.0)?.clamp(20.0, 2000.0),
        voices: r.integer("voices", 6.0, 1, 24)?,
    })
}

/// The layers of sound in a value: a sound, or a list of them (lists inside
/// lists are fine).
fn collect_layers(v: &Val, span: crate::error::Span, out: &mut Vec<std::sync::Arc<crate::sound::Layer>>) -> Res<()> {
    match v {
        Val::Sound(l) => out.push(l.clone()),
        Val::Input(i) if matches!(i.kind, crate::input::Kind::Sound(_)) => {
            out.push(std::sync::Arc::new(crate::sound::Layer::Clip { input: i.clone(), gain: 1.0 }))
        }
        Val::List(items) => {
            for i in items.iter() {
                collect_layers(i, span, out)?;
            }
        }
        _ => {
            return Err(Error::new(
                "expected sounds: `drone(field)`, `notes(pattern)`, `texture(field)` or a sound input, or a list of them",
                span,
            ))
        }
    }
    Ok(())
}

fn mix_of(r: &Render, duration: f64) -> Res<crate::sound::Mix> {
    Ok(crate::sound::Mix {
        duration,
        room: r.number("room", 0.25)?.clamp(0.0, 1.0),
        echo: r.number("echo", 0.0)?.clamp(0.0, 1.0),
    })
}

fn layered(r: &Render) -> bool {
    matches!(r.positional.first(), Some((Val::Sound(_) | Val::List(_), _)))
        || matches!(r.positional.first(), Some((Val::Input(i), _)) if matches!(i.kind, crate::input::Kind::Sound(_)))
}

/// Checks a `render sound` (layered or original) and gives its duration.
pub fn sound_duration(r: &Render) -> Res<f64> {
    if layered(r) {
        r.check(16, &["duration", "room", "echo"])?;
        let mut layers = Vec::new();
        for (v, span) in &r.positional {
            collect_layers(v, *span, &mut layers)?;
        }
        mix_of(r, 0.0)?;
        return r.duration(r.default_length(20.0));
    }
    Ok(sound_spec(r)?.duration)
}

/// Checks a video's `sound:` layers, if it has some.
fn check_video_sound(r: &Render) -> Res<()> {
    if let Some((v, span)) = r.named.get("sound") {
        collect_layers(v, *span, &mut Vec::new())?;
    }
    Ok(())
}

/// The channels of a `render sound`: layers (stereo) when it is given sounds,
/// otherwise the original drone and bells (mono, as before).
pub fn sound_channels(r: &Render, seed: u64) -> Res<Vec<Vec<f64>>> {
    if layered(r) {
        r.check(16, &["duration", "room", "echo"])?;
        let mut layers = Vec::new();
        for (v, span) in &r.positional {
            collect_layers(v, *span, &mut layers)?;
        }
        let [l, rr] = crate::sound::render(&layers, &mix_of(r, r.duration(r.default_length(20.0))?)?);
        return Ok(vec![l, rr]);
    }
    let s = sound_spec(r)?;
    let f = r.required_field(0, "a field (the drone)")?;
    Ok(vec![master(&synth(f, r.pattern_arg()?, &s, seed))])
}

/// The soundtrack of a video: its `sound:` layers, or the original drone and
/// bells of the field and pattern.
pub fn video_channels(r: &Render, spec: &VideoSpec, seed: u64) -> Res<Vec<Vec<f64>>> {
    if let Some((v, span)) = r.named.get("sound") {
        let mut layers = Vec::new();
        collect_layers(v, *span, &mut layers)?;
        let [l, rr] = crate::sound::render(&layers, &mix_of(r, spec.duration)?);
        return Ok(vec![l, rr]);
    }
    let f = r.required_field(0, "the field to animate")?;
    Ok(vec![master(&synth(f, r.pattern_arg()?, &spec.sound, seed))])
}

pub struct MidiSpec {
    pub duration: f64,
    pub tempo: Option<f64>,
    pub layers: Vec<std::sync::Arc<crate::sound::Layer>>,
}

/// `render midi(layers…, duration, tempo)`: the same layers as a sound.
pub fn midi_spec(r: &Render) -> Res<MidiSpec> {
    r.check(16, &["duration", "tempo"])?;
    let mut layers = Vec::new();
    for (v, span) in &r.positional {
        collect_layers(v, *span, &mut layers)?;
    }
    if layers.is_empty() {
        return Err(Error::new("`render midi` plays layers: `render midi(drone(field), notes(pattern), cc(field, 74))`", r.span));
    }
    let tempo = r.number("tempo", 0.0)?;
    if !(0.0..=400.0).contains(&tempo) {
        return Err(Error::new("`tempo` goes from 0 to 400 beats per minute", r.span_of("tempo")));
    }
    Ok(MidiSpec { duration: r.duration(r.default_length(20.0))?, tempo: (tempo > 0.0).then_some(tempo), layers })
}

/// The score of a `render midi`.
pub fn midi_score(r: &Render, title: &str) -> Res<crate::midi::Score> {
    let s = midi_spec(r)?;
    Ok(crate::midi::score(&s.layers, s.duration, s.tempo, title))
}

/// What an edition plays live: its first `render midi`, otherwise the layers
/// of its first layered sound or video soundtrack. None: it has no music.
pub fn live_score(ed: &Edition) -> Option<Res<crate::midi::Score>> {
    if let Some(r) = ed.renders.iter().find(|r| r.kind == "midi") {
        return Some(midi_score(r, &ed.name));
    }
    for r in &ed.renders {
        let found = (|| -> Res<Option<crate::midi::Score>> {
            let (layers, duration) = match r.kind.as_str() {
                "sound" if layered(r) => {
                    let mut l = Vec::new();
                    for (v, span) in &r.positional {
                        collect_layers(v, *span, &mut l)?;
                    }
                    (l, sound_duration(r)?)
                }
                "video" => match r.named.get("sound") {
                    Some((v, span)) => {
                        let mut l = Vec::new();
                        collect_layers(v, *span, &mut l)?;
                        (l, video_spec(r)?.duration)
                    }
                    None => return Ok(None),
                },
                _ => return Ok(None),
            };
            Ok(Some(crate::midi::score(&layers, duration, None, &ed.name)))
        })();
        match found {
            Ok(Some(s)) => return Some(Ok(s)),
            Ok(None) => {}
            Err(e) => return Some(Err(e)),
        }
    }
    None
}

pub fn sound_spec(r: &Render) -> Res<SoundSpec> {
    r.check(2, &["duration", "fundamental", "voices"])?;
    r.required_field(0, "a field (the drone)")?;
    r.pattern_arg()?;
    sound_params(r, r.duration(r.default_length(20.0))?)
}

pub fn video_spec(r: &Render) -> Res<VideoSpec> {
    r.check(2, &["duration", "fps", "width", "height", "palette", "audio", "format", "fundamental", "voices", "sound", "room", "echo", "bands", "range"])?;
    let painting = matches!(r.visual("the field to animate")?, Visual::Paint(_));
    if !painting {
        r.pattern_arg()?;
    }
    let duration = r.duration(r.default_length(10.0))?;
    let fps = r.integer("fps", 30.0, 1, 120)?;
    // Even sizes: required by the yuv420p encoding most players expect.
    let w = r.integer("width", 720.0, 16, 3840)? & !1;
    // an image input: the same proportions
    let h_default = r.canvas.map_or(w as f64, |(iw, ih)| (w as f64 * ih as f64 / iw as f64).round().max(16.0));
    let h = r.integer("height", h_default, 16, 3840)? & !1;
    let format = r.string("format", "mp4")?;
    if !matches!(format.as_str(), "mp4" | "gif") {
        return Err(Error::new("`format` must be \"mp4\" or \"gif\"", r.span_of("format")));
    }
    let gif = format == "gif";
    // A painting has no sound of its own: only `sound:` layers give it one.
    let quiet = gif || (painting && !r.named.contains_key("sound"));
    let audio = r.string("audio", if quiet { "off" } else { "on" })?;
    if painting && audio == "on" && !r.named.contains_key("sound") {
        return Err(Error::new("a painting has no sound of its own: give the video `sound: [drone(…), notes(…)]`", r.span_of("audio")));
    }
    if !matches!(audio.as_str(), "on" | "off") {
        return Err(Error::new("`audio` must be \"on\" or \"off\"", r.span_of("audio")));
    }
    if gif && audio == "on" {
        return Err(Error::new(
            "a gif has no sound: remove `audio: \"on\"` or use `format: \"mp4\"`",
            r.span_of("audio"),
        ));
    }
    Ok(VideoSpec {
        w,
        h,
        fps,
        duration,
        stops: r.palette()?,
        range: r.range()?,
        gif,
        audio: audio == "on",
        sound: sound_params(r, duration)?,
    })
}

pub fn sculpture_spec(r: &Render) -> Res<SculptureSpec> {
    r.check(1, &["resolution", "relief", "side", "base", "at"])?;
    r.required_field(0, "the field to sculpt")?;
    Ok(SculptureSpec {
        n: r.integer("resolution", 160.0, 8, 1024)?,
        relief: r.number("relief", 0.25)?,
        side: r.number("side", 100.0)?,
        base: r.number("base", 4.0)?.max(0.5),
        at: r.number("at", 0.0)?,
    })
}

/// Checks every render of an edition, so mistakes show up before any drawing starts.
pub fn validate(ed: &Edition) -> Res<()> {
    for r in &ed.renders {
        match r.kind.as_str() {
            "image" => drop(image_spec(r)?),
            "sound" => drop(sound_duration(r)?),
            "video" => {
                video_spec(r)?;
                check_video_sound(r)?;
            }
            "midi" => drop(midi_spec(r)?),
            _ => drop(sculpture_spec(r)?),
        }
    }
    Ok(())
}

// ───────────────────────────── colour ─────────────────────────────

pub const PALETTES: &[(&str, &[[u8; 3]])] = &[
    ("ink", &[[241, 236, 224], [150, 140, 135], [38, 36, 50]]),
    ("sea", &[[6, 20, 40], [18, 80, 120], [110, 180, 195], [240, 245, 238]]),
    ("ember", &[[10, 5, 5], [115, 20, 12], [225, 105, 25], [255, 232, 175]]),
    ("lichen", &[[28, 33, 24], [88, 108, 58], [186, 196, 136], [240, 240, 220]]),
    ("night", &[[4, 4, 14], [55, 28, 88], [196, 118, 176], [250, 240, 250]]),
];

/// How values become colours: colours spread evenly or placed at values,
/// blended between, and optionally posterized into `bands` flat levels.
#[derive(Clone, Debug)]
pub struct Palette {
    pub colors: Vec<[u8; 3]>,
    /// Where each colour sits, from 0 to 1 (in order); none: evenly spread.
    pub positions: Option<Vec<f64>>,
    /// 0: smooth; n ≥ 2: n flat levels, from the first colour to the last.
    pub bands: usize,
}

impl Palette {
    pub fn named(name: &str) -> Option<Palette> {
        PALETTES.iter().find(|(n, _)| *n == name).map(|(_, s)| Palette { colors: s.to_vec(), positions: None, bands: 0 })
    }

    /// The colour of a value from 0 to 1.
    pub fn color(&self, v: f64) -> [u8; 3] {
        let v = match self.bands {
            0 | 1 => v,
            n => (((v * n as f64).floor().max(0.0) as usize).min(n - 1)) as f64 / (n - 1) as f64,
        };
        let c = &self.colors;
        if c.len() == 1 {
            return c[0];
        }
        match &self.positions {
            None => color(c, v),
            Some(p) => {
                if v <= p[0] {
                    return c[0];
                }
                let last = p.len() - 1;
                if v >= p[last] {
                    return c[last];
                }
                // the last stop at or below v (two stops at one place: a sharp edge)
                let k = (0..last).rev().find(|&k| p[k] <= v).unwrap_or(0);
                let span = p[k + 1] - p[k];
                let f = if span > 0.0 { (v - p[k]) / span } else { 1.0 };
                let (a, b) = (c[k], c[k + 1]);
                [0, 1, 2].map(|i| (a[i] as f64 + (b[i] as f64 - a[i] as f64) * f).round() as u8)
            }
        }
    }
}

/// "#rgb" or "#rrggbb" (the "#" may be left out).
pub fn parse_color(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#').unwrap_or(s);
    let hex = |t: &str| u8::from_str_radix(t, 16).ok();
    match h.len() {
        3 => {
            let d: Vec<u8> = h.chars().map(|ch| hex(&ch.to_string())).collect::<Option<Vec<_>>>()?;
            Some([d[0] * 17, d[1] * 17, d[2] * 17])
        }
        6 => Some([hex(&h[0..2])?, hex(&h[2..4])?, hex(&h[4..6])?]),
        _ => None,
    }
}

/// A palette from a value: a name ("ember"), a list of colours, or a list of
/// [position, colour] pairs.
pub fn palette_from(v: &Val, span: crate::error::Span, bands: usize) -> Res<Palette> {
    let names = || PALETTES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ");
    let bad_color = |s: &str| Error::new(format!("`{s}` is not a colour: write it \"#rrggbb\" or \"#rgb\""), span);
    let mut p = match v {
        Val::Str(name) => match Palette::named(name) {
            Some(p) => p,
            None if parse_color(name).is_some() => {
                return Err(Error::new(format!("one colour is not a palette: give a list, [\"{name}\", \"#ffffff\"]"), span))
            }
            None => return Err(Error::new(format!("unknown palette `{name}` — available: {}, or a list of colours", names()), span)),
        },
        Val::List(items) if !items.is_empty() => {
            let mut colors = Vec::new();
            let mut positions = Vec::new();
            for item in items.iter() {
                match item {
                    Val::Str(c) => colors.push(parse_color(c).ok_or_else(|| bad_color(c))?),
                    Val::List(pair) if pair.len() == 2 => {
                        let at = match &pair[0] {
                            Val::Field(f) => f.constant(),
                            _ => None,
                        }
                        .ok_or_else(|| Error::new("a placed colour is [position, \"#rrggbb\"], the position a fixed number", span))?;
                        let c = match &pair[1] {
                            Val::Str(c) => parse_color(c).ok_or_else(|| bad_color(c))?,
                            _ => return Err(Error::new("a placed colour is [position, \"#rrggbb\"]", span)),
                        };
                        positions.push(at);
                        colors.push(c);
                    }
                    _ => return Err(Error::new("a palette is a list of colours (\"#rrggbb\") or of [position, colour]", span)),
                }
            }
            if !positions.is_empty() && positions.len() != colors.len() {
                return Err(Error::new("in a palette, give every colour a position, or none", span));
            }
            if positions.windows(2).any(|w| w[1] < w[0]) {
                return Err(Error::new("the positions of a palette must go up (equal ones make a sharp edge)", span));
            }
            Palette { colors, positions: if positions.is_empty() { None } else { Some(positions) }, bands: 0 }
        }
        _ => return Err(Error::new(format!("`palette` is a name ({}) or a list of colours", names()), span)),
    };
    p.bands = bands;
    Ok(p)
}

pub fn color(stops: &[[u8; 3]], v: f64) -> [u8; 3] {
    let pos = v * (stops.len() - 1) as f64;
    let k = (pos.floor() as usize).min(stops.len() - 2);
    let f = pos - k as f64;
    let (a, b) = (stops[k], stops[k + 1]);
    [0, 1, 2].map(|c| (a[c] as f64 + (b[c] as f64 - a[c] as f64) * f).round() as u8)
}

/// Robust bounds (0.5 %–99.5 % percentiles) so a single spike doesn't flatten everything.
pub fn bounds(vals: &[f64]) -> (f64, f64) {
    let mut v: Vec<f64> = vals.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return (0.0, 1.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
    (q(0.005), q(0.995))
}

pub fn normalize(x: f64, (lo, hi): (f64, f64)) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    // A field that is the same everywhere is shown as it is: empty stays blank.
    if hi - lo < 1e-12 { x.clamp(0.0, 1.0) } else { ((x - lo) / (hi - lo)).clamp(0.0, 1.0) }
}

// ───────────────────────────── frames ─────────────────────────────

/// Samples a field over a w×h frame at time `t`. The unit square [0, 1]² is
/// always fully visible and centred: a wider or taller frame shows more around it.
pub fn sample_frame(f: &Field, w: usize, h: usize, t: f64, seed: u64) -> Vec<f64> {
    // What doesn't depend on the position is computed once for the frame.
    sample_frame_raw(&f.at_time(t, seed), w, h, t, seed)
}

/// `ALEA_TREE=1` evaluates tiles by walking the tree instead of compiling them
/// (for comparison: the pixels are the same).
fn tree_walk() -> bool {
    #[cfg(feature = "native")]
    {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ON.get_or_init(|| std::env::var("ALEA_TREE").is_ok())
    }
    #[cfg(not(feature = "native"))]
    false
}

pub fn sample_frame_raw(f: &Field, w: usize, h: usize, t: f64, seed: u64) -> Vec<f64> {
    use crate::bounds::{Culler, Iv, Tile};
    // The frame is computed in tiles; for each tile the field is first
    // simplified to what can actually change there (see bounds.rs).
    const TS: usize = 32;
    let s = w.min(h) as f64;
    let (ox, oy) = (0.5 - w as f64 / (2.0 * s), 0.5 - h as f64 / (2.0 * s));
    let cx = |px: usize| ox + (px as f64 + 0.5) / s;
    let cy = |py: usize| oy + (py as f64 + 0.5) / s;
    par_map(h.div_ceil(TS), |band| {
        let (y0, y1) = (band * TS, ((band + 1) * TS).min(h));
        let mut out = vec![0.0; (y1 - y0) * w];
        for x0 in (0..w).step_by(TS) {
            let x1 = (x0 + TS).min(w);
            let tile = Tile { x: Iv { lo: cx(x0), hi: cx(x1 - 1) }, y: Iv { lo: cy(y0), hi: cy(y1 - 1) }, t };
            let local = Culler::new().fold(f, tile, seed);
            if tree_walk() {
                for py in y0..y1 {
                    let y = cy(py);
                    for px in x0..x1 {
                        out[(py - y0) * w + px] = local.eval([cx(px), y, t], seed);
                    }
                }
                continue;
            }
            // The simplified tile, compiled once, then run for each pixel.
            let prog = crate::compile::Program::new(&local, seed);
            let mut slots = prog.slots(t);
            for py in y0..y1 {
                let y = cy(py);
                for px in x0..x1 {
                    out[(py - y0) * w + px] = prog.run(&mut slots, cx(px), y);
                }
            }
        }
        out
    })
    .concat()
}

/// One set of bounds for a whole animation, so colours don't flicker between
/// frames: a few low-resolution frames are sampled across time first.
pub fn animation_bounds(f: &Field, w: usize, h: usize, seed: u64) -> (f64, f64) {
    let ph = (96 * h / w.max(1)).max(1);
    let probe: Vec<f64> = (0..24).flat_map(|k| sample_frame(f, 96, ph, k as f64 / 24.0, seed)).collect();
    bounds(&probe)
}

/// RGB bytes of one frame. Without `fixed` bounds, the frame is normalized on its own.
pub fn frame_rgb(f: &Field, w: usize, h: usize, t: f64, seed: u64, stops: &Palette, fixed: Option<(f64, f64)>) -> Vec<u8> {
    let vals = sample_frame(f, w, h, t, seed);
    let b = fixed.unwrap_or_else(|| bounds(&vals));
    let mut rgb = Vec::with_capacity(w * h * 3);
    for v in vals {
        rgb.extend_from_slice(&stops.color(normalize(v, b)));
    }
    rgb
}

// ───────────────────────────── sound ─────────────────────────────

pub const SAMPLE_RATE: u32 = 44_100;
const RATIOS: [f64; 5] = [1.0, 9.0 / 8.0, 5.0 / 4.0, 3.0 / 2.0, 5.0 / 3.0]; // pentatonic

/// The field drives a drone: each voice reads the field along a horizontal
/// line (x = 0.5, y = its pitch) over time. An optional pattern becomes
/// percussive grains: t → onset, y → pitch, size → duration.
pub fn synth(f: &Field, pattern: Option<&[Event]>, spec: &SoundSpec, seed: u64) -> Vec<f64> {
    let sr = SAMPLE_RATE as f64;
    let SoundSpec { duration, fundamental, voices } = *spec;

    // Envelopes at control rate (200 Hz).
    let nk = (duration * 200.0) as usize + 2;
    let raw: Vec<Vec<f64>> = par_map(voices, |v| {
        let y = (v as f64 + 0.5) / voices as f64;
        (0..nk).map(|k| f.eval([0.5, y, k as f64 / (nk - 1) as f64], seed)).collect()
    });
    let b = bounds(&raw.concat());
    let env: Vec<Vec<f64>> = raw.iter().map(|l| l.iter().map(|x| normalize(*x, b).powi(2)).collect()).collect();

    let n = (duration * sr) as usize;
    let mut buf = vec![0.0f64; n];
    let voice_amp = 0.35 / (voices as f64).sqrt();
    for (v, e) in env.iter().enumerate() {
        let freq = fundamental * RATIOS[v % 5] * 2f64.powi((v / 5) as i32);
        let (w1, w2) = (TAU * freq / sr, TAU * freq * 1.003 / sr);
        for (s, out) in buf.iter_mut().enumerate() {
            let pos = s as f64 / n as f64 * (nk - 1) as f64;
            let k = (pos as usize).min(nk - 2);
            let a = e[k] + (e[k + 1] - e[k]) * (pos - k as f64);
            *out += voice_amp * a * (libm::sin(w1 * s as f64) + 0.5 * libm::sin(w2 * s as f64));
        }
    }

    for e in pattern.unwrap_or(&[]) {
        let onset = (e.t.clamp(0.0, 1.0) * duration * sr) as usize;
        let freq = fundamental * libm::pow(2.0, 1.0 + (1.0 - e.y.clamp(0.0, 1.0)) * 3.0);
        let d = (0.08 + e.size * 20.0).min(3.0);
        let amp = e.intensity.clamp(0.0, 4.0) * 0.25;
        let len = ((d * sr) as usize).min(n.saturating_sub(onset));
        for k in 0..len {
            let tt = k as f64 / sr;
            let attack = (tt / 0.005).min(1.0);
            let env = attack * libm::exp(-tt * 5.0 / d);
            let w = TAU * freq * tt;
            buf[onset + k] += amp * env * (libm::sin(w) + 0.3 * libm::sin(w * 2.76)); // bell-like timbre
        }
    }
    buf
}

/// Normalizes, soft-clips and fades in and out: samples ready to play, in [-1, 1].
pub fn master(buf: &[f64]) -> Vec<f64> {
    let n = buf.len();
    let peak = buf.iter().fold(0.0f64, |m, x| m.max(x.abs())).max(1e-9);
    let fade = 0.5 * SAMPLE_RATE as f64;
    buf.iter()
        .enumerate()
        .map(|(s, x)| {
            let g = (s.min(n - 1 - s) as f64 / fade).min(1.0);
            libm::tanh(1.5 * x / peak) / libm::tanh(1.5) * 0.9 * g
        })
        .collect()
}

// ───────────────────────────── sculpture ─────────────────────────────

pub type Tri = [[f32; 3]; 3];

/// Heights of the relief, normalized to [0, 1], on an n×n grid.
pub fn heights(f: &Field, spec: &SculptureSpec, seed: u64) -> Vec<f64> {
    let n = spec.n;
    let raw = par_map(n * n, |k| {
        let (i, j) = (k % n, k / n);
        f.eval([i as f64 / (n - 1) as f64, j as f64 / (n - 1) as f64, spec.at], seed)
    });
    let b = bounds(&raw);
    raw.into_iter().map(|v| normalize(v, b)).collect()
}

/// The relief sitting on a base: a closed solid, ready for a 3D-printing slicer (mm).
pub fn sculpture_mesh(hts: &[f64], spec: &SculptureSpec) -> Vec<Tri> {
    let SculptureSpec { n, relief, side, base, .. } = *spec;
    let step = side / (n - 1) as f64;
    let top = |i: usize, j: usize| -> [f32; 3] {
        let z = base + hts[j * n + i] * relief * side;
        [(i as f64 * step) as f32, (j as f64 * step) as f32, z as f32]
    };
    let bottom = |i: usize, j: usize| -> [f32; 3] { [(i as f64 * step) as f32, (j as f64 * step) as f32, 0.0] };

    let mut tris: Vec<Tri> = Vec::with_capacity(4 * n * n + 8 * n);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let (a, b, c, d) = (top(i, j), top(i + 1, j), top(i + 1, j + 1), top(i, j + 1));
            tris.push([a, b, c]);
            tris.push([a, c, d]);
            let (a, b, c, d) = (bottom(i, j), bottom(i + 1, j), bottom(i + 1, j + 1), bottom(i, j + 1));
            tris.push([a, c, b]);
            tris.push([a, d, c]);
        }
    }
    for k in 0..n - 1 {
        // front (y = 0) and back (y = max)
        let (a, b, a0, b0) = (top(k, 0), top(k + 1, 0), bottom(k, 0), bottom(k + 1, 0));
        tris.push([a0, b0, b]);
        tris.push([a0, b, a]);
        let (a, b, a0, b0) = (top(k, n - 1), top(k + 1, n - 1), bottom(k, n - 1), bottom(k + 1, n - 1));
        tris.push([a0, b, b0]);
        tris.push([a0, a, b]);
        // left (x = 0) and right (x = max)
        let (p, q, p0, q0) = (top(0, k), top(0, k + 1), bottom(0, k), bottom(0, k + 1));
        tris.push([p0, q, q0]);
        tris.push([p0, p, q]);
        let (p, q, p0, q0) = (top(n - 1, k), top(n - 1, k + 1), bottom(n - 1, k), bottom(n - 1, k + 1));
        tris.push([p0, q0, q]);
        tris.push([p0, q, p]);
    }
    tris
}

/// Binary STL.
pub fn stl_bytes(tris: &[Tri], seed: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + tris.len() * 50);
    let mut header = format!("alea - edition {seed:016x}").into_bytes();
    header.resize(80, b' ');
    out.extend_from_slice(&header);
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for [a, b, c] in tris {
        let (u, v) = ([0, 1, 2].map(|k| b[k] - a[k]), [0, 1, 2].map(|k| c[k] - a[k]));
        let mut nrm = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let l = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt().max(1e-12);
        nrm.iter_mut().for_each(|x| *x /= l);
        for p in [nrm, *a, *b, *c] {
            for x in p {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

/// A shaded picture of the relief (light from the top left), to preview a sculpture.
pub fn relief_preview(hts: &[f64], spec: &SculptureSpec) -> Vec<u8> {
    let n = spec.n;
    let k = spec.relief * (n - 1) as f64; // slope scale: height units per cell
    let h = |i: usize, j: usize| hts[j.min(n - 1) * n + i.min(n - 1)];
    let light = {
        let l = [-1.0f64, -1.0, 1.4];
        let m = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        [l[0] / m, l[1] / m, l[2] / m]
    };
    let mut rgb = Vec::with_capacity(n * n * 3);
    for j in 0..n {
        for i in 0..n {
            let dx = (h(i + 1, j) - h(i.saturating_sub(1), j)) * k / 2.0;
            let dy = (h(i, j + 1) - h(i, j.saturating_sub(1))) * k / 2.0;
            let m = (dx * dx + dy * dy + 1.0).sqrt();
            let shade = ((-dx * light[0] - dy * light[1] + light[2]) / m).clamp(0.0, 1.0);
            let v = (40.0 + 200.0 * shade + 15.0 * h(i, j)) as u8;
            rgb.extend_from_slice(&[v, v.saturating_sub(4), v.saturating_sub(12)]);
        }
    }
    rgb
}
