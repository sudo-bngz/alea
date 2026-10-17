//! Sound design: layers of sound made from fields and patterns.
//!
//! A soundtrack is a list of layers, each with its own role:
//!
//! - `drone(field)`: voices that hold notes of a scale; each voice listens to
//!   the field along one row of the canvas, on the left and on the right, and
//!   is loud (and brighter) where the field is high.
//! - `notes(pattern)`: each event plays one note of an instrument; its height
//!   on the canvas picks the note in the scale, its x places it left or right.
//! - `texture(field)`: an ambience (wind, waves, rain, crackle) whose
//!   loudness follows how much of the field is lit.
//!
//! The layers are mixed, sent through an echo and a room, and mastered in
//! stereo. Everything is computed with the same arithmetic on every platform,
//! and every random detail comes from the edition's seed.

use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use crate::eval::{Event, Field};
use crate::harmony::{Arp, Harmony};
use crate::synth::{NoteCtx, Synth};

pub const SR: f64 = 44_100.0;

// ───────────────────────────── vocabulary ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Voice {
    Sine,
    Organ,
    Reed,
    Strings,
    Choir,
    Glass,
    Wind,
}

pub const DRONE_INSTRUMENTS: &[&str] = &["sine", "organ", "reed", "strings", "choir", "glass", "wind"];

impl Voice {
    pub fn parse(s: &str) -> Option<Voice> {
        Some(match s {
            "sine" => Voice::Sine,
            "organ" => Voice::Organ,
            "reed" => Voice::Reed,
            "strings" => Voice::Strings,
            "choir" => Voice::Choir,
            "glass" => Voice::Glass,
            "wind" => Voice::Wind,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Strike {
    Bell,
    Pluck,
    Marimba,
    Kalimba,
    Glass,
    Wood,
    Drum,
    Tick,
}

pub const NOTE_INSTRUMENTS: &[&str] = &["bell", "pluck", "marimba", "kalimba", "glass", "wood", "drum", "tick"];

impl Strike {
    pub fn parse(s: &str) -> Option<Strike> {
        Some(match s {
            "bell" => Strike::Bell,
            "pluck" => Strike::Pluck,
            "marimba" => Strike::Marimba,
            "kalimba" => Strike::Kalimba,
            "glass" => Strike::Glass,
            "wood" => Strike::Wood,
            "drum" => Strike::Drum,
            "tick" => Strike::Tick,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ambience {
    Wind,
    Waves,
    Rain,
    Crackle,
}

pub const TEXTURES: &[&str] = &["wind", "waves", "rain", "crackle"];

impl Ambience {
    pub fn parse(s: &str) -> Option<Ambience> {
        Some(match s {
            "wind" => Ambience::Wind,
            "waves" => Ambience::Waves,
            "rain" => Ambience::Rain,
            "crackle" => Ambience::Crackle,
            _ => return None,
        })
    }
}

/// A scale: semitones above the key, within one octave; or the harmonic series.
#[derive(Clone, Debug, PartialEq)]
pub enum Scale {
    Steps(Vec<f64>),
    Harmonics,
}

pub const SCALES: &[&str] = &[
    "pentatonic", "minor-pentatonic", "major", "minor", "dorian", "phrygian", "lydian", "mixolydian",
    "harmonic-minor", "whole-tone", "blues", "hirajoshi", "chromatic", "harmonics",
];

impl Scale {
    pub fn parse(s: &str) -> Option<Scale> {
        let steps: &[f64] = match s {
            "pentatonic" => &[0., 2., 4., 7., 9.],
            "minor-pentatonic" => &[0., 3., 5., 7., 10.],
            "major" => &[0., 2., 4., 5., 7., 9., 11.],
            "minor" => &[0., 2., 3., 5., 7., 8., 10.],
            "dorian" => &[0., 2., 3., 5., 7., 9., 10.],
            "phrygian" => &[0., 1., 3., 5., 7., 8., 10.],
            "lydian" => &[0., 2., 4., 6., 7., 9., 11.],
            "mixolydian" => &[0., 2., 4., 5., 7., 9., 10.],
            "harmonic-minor" => &[0., 2., 3., 5., 7., 8., 11.],
            "whole-tone" => &[0., 2., 4., 6., 8., 10.],
            "blues" => &[0., 3., 5., 6., 7., 10.],
            "hirajoshi" => &[0., 2., 3., 7., 8.],
            "chromatic" => &[0., 1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11.],
            "harmonics" => return Some(Scale::Harmonics),
            _ => return None,
        };
        Some(Scale::Steps(steps.to_vec()))
    }

    pub fn len(&self) -> usize {
        match self {
            Scale::Steps(s) => s.len(),
            Scale::Harmonics => 8,
        }
    }

    /// The frequency of degree `d` (0 is the key itself; it climbs octave after octave).
    pub fn freq(&self, key: f64, d: usize) -> f64 {
        match self {
            Scale::Steps(s) => {
                let (oct, k) = (d / s.len(), d % s.len());
                key * libm::pow(2.0, (s[k] + 12.0 * oct as f64) / 12.0)
            }
            // Partials 1 to 16 of the key, then the same an octave up.
            Scale::Harmonics => {
                let (oct, k) = (d / 8, d % 8);
                key * (k as f64 + 1.0) * libm::pow(2.0, oct as f64)
            }
        }
    }
}

/// How a `notes` layer plays its events, beyond the scale: a progression
/// to follow, an arpeggio, swing and human looseness.
#[derive(Clone, Debug)]
pub struct Feel {
    /// 0.5: straight; up to 0.75: every second sixteenth comes later.
    pub swing: f64,
    /// 0: exact; 1: moments up to 30 ms off, velocities ±40 %.
    pub humanize: f64,
    pub harmony: Option<Arc<Harmony>>,
    pub arp: Option<Arp>,
}

impl Default for Feel {
    fn default() -> Self {
        Feel { swing: 0.5, humanize: 0.0, harmony: None, arp: None }
    }
}

impl Feel {
    fn plain(&self) -> bool {
        self.swing == 0.5 && self.humanize == 0.0 && self.harmony.is_none() && self.arp.is_none()
    }
}

/// One layer of a soundtrack.
#[derive(Debug)]
pub enum Layer {
    /// `synth`: a synth written in alea plays the voices instead of `voice`.
    Drone {
        field: Field,
        seed: u64,
        voices: usize,
        voice: Voice,
        scale: Scale,
        key: f64,
        gain: f64,
        harmony: Option<Arc<Harmony>>,
        synth: Option<Arc<Synth>>,
    },
    /// `synth`: a synth written in alea plays the notes instead of `strike`.
    Notes { events: Arc<Vec<Event>>, strike: Strike, scale: Scale, key: f64, gain: f64, tempo: f64, seed: u64, feel: Feel, synth: Option<Arc<Synth>> },
    Texture { field: Field, seed: u64, kind: Ambience, gain: f64 },
    /// A sound that came in (`input()`), played as it is, dry.
    Clip { input: Arc<crate::input::Input>, gain: f64 },
    /// `cc(field, number)`: a MIDI controller, silent in alea's own sound.
    Control { field: Field, seed: u64, number: u8, channel: u8 },
}

pub struct Mix {
    pub duration: f64,
    pub room: f64,
    pub echo: f64,
}

// ───────────────────────────── tools ─────────────────────────────

/// Deterministic noise, the same on every platform.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }
    fn unit(&mut self) -> f64 {
        0.5 * (self.next() + 1.0)
    }
}

/// A two-pole resonant filter (state-variable): low-pass and band-pass outputs.
#[derive(Default, Clone, Copy)]
struct Svf {
    low: f64,
    band: f64,
}

impl Svf {
    fn tick(&mut self, x: f64, cutoff: f64, q: f64) -> (f64, f64) {
        let f = 2.0 * libm::sin(PI * (cutoff / SR).min(0.24));
        let high = x - self.low - self.band / q;
        self.band += f * high;
        self.low += f * self.band;
        (self.low, self.band)
    }
}

fn pan(p: f64) -> (f64, f64) {
    let a = p.clamp(0.0, 1.0) * PI / 2.0;
    (libm::cos(a), libm::sin(a))
}

/// Linear interpolation in a control-rate envelope, `pos` from 0 to 1.
fn at(env: &[f64], pos: f64) -> f64 {
    let p = pos.clamp(0.0, 1.0) * (env.len() - 1) as f64;
    let k = (p as usize).min(env.len() - 2);
    env[k] + (env[k + 1] - env[k]) * (p - k as f64)
}

fn robust_bounds(v: &[f64]) -> (f64, f64) {
    crate::media::bounds(v)
}

// ───────────────────────────── the layers ─────────────────────────────

/// The partials of a held voice: (ratio to the note, loudness).
fn partials(v: Voice) -> Vec<(f64, f64)> {
    match v {
        Voice::Sine => vec![(1.0, 1.0), (1.003, 0.5)],
        Voice::Organ => vec![(1.0, 1.0), (2.0, 0.6), (3.0, 0.4), (4.0, 0.25), (6.0, 0.15), (8.0, 0.1)],
        Voice::Reed => (0..6).map(|k| ((2 * k + 1) as f64, 1.0 / (2 * k + 1) as f64)).collect(),
        Voice::Strings => (1..=10).map(|k| (k as f64, 1.0 / k as f64)).collect(),
        Voice::Glass => vec![(1.0, 1.0), (2.32, 0.5), (4.25, 0.3), (6.63, 0.18)],
        Voice::Choir | Voice::Wind => vec![],
    }
}

/// How loud a sung harmonic at `f` Hz is, through the formants of a vowel
/// that moves from "oo" (closed) to "ah" (open) as `open` goes from 0 to 1.
fn vowel(f: f64, open: f64) -> f64 {
    let lerp = |a: f64, b: f64| a + (b - a) * open;
    let formants = [(lerp(325.0, 800.0), 1.0, 80.0), (lerp(700.0, 1150.0), 0.5, 90.0), (lerp(2530.0, 2900.0), 0.18, 120.0)];
    formants.iter().map(|(c, a, w)| a * libm::exp(-((f - c) / w).powi(2))).sum::<f64>() + 0.02
}

/// How loud each voice of a drone is along the sound, on the left and on the
/// right: 200 values per second, from 0 to 1 (the field, normalized, squared).
pub fn drone_envelopes(field: &Field, seed: u64, voices: usize, duration: f64) -> Vec<[Vec<f64>; 2]> {
    // Envelopes at control rate: each voice listens on the left and on the right.
    let nk = (duration * 200.0) as usize + 2;
    let raw: Vec<[Vec<f64>; 2]> = crate::media::par_map(voices, |v| {
        let y = (v as f64 + 0.5) / voices as f64;
        let side = |x: f64| (0..nk).map(|k| field.eval([x, y, k as f64 / (nk - 1) as f64], seed)).collect::<Vec<_>>();
        [side(0.3), side(0.7)]
    });
    let all: Vec<f64> = raw.iter().flat_map(|s| s[0].iter().chain(s[1].iter()).copied()).collect();
    let b = robust_bounds(&all);
    let env: Vec<[Vec<f64>; 2]> = raw
        .iter()
        .map(|s| [0, 1].map(|c| s[c].iter().map(|x| crate::media::normalize(*x, b).powi(2)).collect()))
        .collect();
    env
}

/// The degree of the scale a drone's voice `v` holds: chords rather than
/// runs, every other degree for scales of six notes or more.
pub fn drone_degree(scale: &Scale, v: usize) -> usize {
    let step = if scale.len() >= 6 { 2 } else { 1 };
    v * step
}

fn drone(field: &Field, seed: u64, voices: usize, voice: Voice, scale: &Scale, key: f64, n: usize, duration: f64) -> [Vec<f64>; 2] {
    let env = drone_envelopes(field, seed, voices, duration);
    let parts = partials(voice);
    let out: Vec<[Vec<f64>; 2]> = crate::media::par_map(voices, |v| {
        let freq = scale.freq(key, drone_degree(scale, v));
        let mut noise = Noise(seed ^ (v as u64 + 1).wrapping_mul(0x51ED_270B));
        let (mut fl, mut fr) = (Svf::default(), Svf::default());
        let vib_phase = v as f64 * 1.7;
        let mut weights = [0.0f64; 24];
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for s in 0..n {
            let tt = s as f64 / SR;
            let pos = s as f64 / n as f64;
            let (nl, nr) = (at(&env[v][0], pos), at(&env[v][1], pos));
            let e = 0.5 * (nl + nr);
            // where it sounds: towards the side where the field is higher
            let (gl, gr) = pan(0.5 + 0.35 * (nr - nl));
            let (el, er) = (e * gl * std::f64::consts::SQRT_2, e * gr * std::f64::consts::SQRT_2);
            // Louder is also brighter: upper partials open with the field.
            let bright = 0.3 + 0.7 * e;
            let sample = match voice {
                Voice::Wind => {
                    let x = noise.next();
                    let (_, bl) = fl.tick(x, freq, 14.0);
                    let (_, br) = fr.tick(noise.next(), freq * 1.004, 14.0);
                    l[s] = bl * el * 3.0;
                    r[s] = br * er * 3.0;
                    continue;
                }
                Voice::Choir => {
                    // the vowel opens with the field (updated every 64 samples)
                    if s % 64 == 0 {
                        for (h, w) in weights.iter_mut().enumerate() {
                            let fh = freq * (h + 1) as f64;
                            *w = if fh > 5000.0 { 0.0 } else { vowel(fh, e) };
                        }
                    }
                    // vibrato: the pitch wavers by 0.3 % (as a phase, so it never drifts)
                    let wobble = 0.003 * freq / 5.0 * libm::sin(TAU * 5.0 * tt + vib_phase);
                    let mut acc = 0.0;
                    for (h, w) in weights.iter().enumerate() {
                        if *w > 0.0 {
                            let k = (h + 1) as f64;
                            acc += w * libm::sin(TAU * freq * k * tt + k * wobble + h as f64);
                        }
                    }
                    acc * 0.5
                }
                Voice::Strings => {
                    // three players, slightly apart, with a slow vibrato
                    let mut acc = 0.0;
                    for (d, ph) in [(1.0, 0.0), (1.004, 2.1), (0.9965, 4.2)] {
                        let f0 = freq * d;
                        let wobble = 0.004 * f0 / 5.2 * libm::sin(TAU * 5.2 * tt + vib_phase + ph);
                        let mut g = 1.0;
                        for (ratio, amp) in &parts {
                            acc += amp * g * libm::sin(TAU * f0 * ratio * tt + ratio * (wobble + ph));
                            g *= bright;
                        }
                    }
                    acc / 3.0
                }
                _ => {
                    let mut acc = 0.0;
                    let mut g = 1.0;
                    for (ratio, amp) in &parts {
                        acc += amp * g * libm::sin(TAU * freq * ratio * tt);
                        g *= bright;
                    }
                    acc
                }
            };
            l[s] = sample * el;
            r[s] = sample * er;
        }
        [l, r]
    });
    let mut mix = [vec![0.0; n], vec![0.0; n]];
    for o in out {
        for c in 0..2 {
            for (m, x) in mix[c].iter_mut().zip(&o[c]) {
                *m += x;
            }
        }
    }
    mix
}

/// A drone whose voices follow a progression: each glides to its next chord
/// tone at each change. The same voices as `drone`, with their phase carried
/// from sample to sample so the pitch can move.
#[allow(clippy::too_many_arguments)]
fn drone_moving(field: &Field, seed: u64, voices: usize, voice: Voice, key: f64, harmony: &Harmony, n: usize, duration: f64) -> [Vec<f64>; 2] {
    let env = drone_envelopes(field, seed, voices, duration);
    let parts = partials(voice);
    let plan = harmony.voicing(voices, key);
    let glide = 1.0 - libm::exp(-1.0 / (0.06 * SR));
    let out: Vec<[Vec<f64>; 2]> = crate::media::par_map(voices, |v| {
        let target = |pos: f64| key * libm::pow(2.0, crate::harmony::pitch_at(&plan[v], pos) / 12.0);
        let mut freq = target(0.0);
        let mut ph = 0.0f64; // the fundamental's phase, in turns
        let mut noise = Noise(seed ^ (v as u64 + 1).wrapping_mul(0x51ED_270B));
        let (mut fl, mut fr) = (Svf::default(), Svf::default());
        let vib_phase = v as f64 * 1.7;
        let mut weights = [0.0f64; 24];
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for s in 0..n {
            let tt = s as f64 / SR;
            let pos = s as f64 / n as f64;
            freq += (target(pos) - freq) * glide;
            ph += freq / SR;
            let (nl, nr) = (at(&env[v][0], pos), at(&env[v][1], pos));
            let e = 0.5 * (nl + nr);
            let (gl, gr) = pan(0.5 + 0.35 * (nr - nl));
            let (el, er) = (e * gl * std::f64::consts::SQRT_2, e * gr * std::f64::consts::SQRT_2);
            let bright = 0.3 + 0.7 * e;
            let sample = match voice {
                Voice::Wind => {
                    let (_, bl) = fl.tick(noise.next(), freq, 14.0);
                    let (_, br) = fr.tick(noise.next(), freq * 1.004, 14.0);
                    l[s] = bl * el * 3.0;
                    r[s] = br * er * 3.0;
                    continue;
                }
                Voice::Choir => {
                    if s % 64 == 0 {
                        for (h, w) in weights.iter_mut().enumerate() {
                            let fh = freq * (h + 1) as f64;
                            *w = if fh > 5000.0 { 0.0 } else { vowel(fh, e) };
                        }
                    }
                    let wobble = 0.003 * freq / 5.0 * libm::sin(TAU * 5.0 * tt + vib_phase);
                    let mut acc = 0.0;
                    for (h, w) in weights.iter().enumerate() {
                        if *w > 0.0 {
                            let k = (h + 1) as f64;
                            acc += w * libm::sin(TAU * ph * k + k * wobble + h as f64);
                        }
                    }
                    acc * 0.5
                }
                Voice::Strings => {
                    let mut acc = 0.0;
                    for (d, phd) in [(1.0, 0.0), (1.004, 2.1), (0.9965, 4.2)] {
                        let wobble = 0.004 * freq * d / 5.2 * libm::sin(TAU * 5.2 * tt + vib_phase + phd);
                        let mut g = 1.0;
                        for (ratio, amp) in &parts {
                            acc += amp * g * libm::sin(TAU * ph * d * ratio + ratio * (wobble + phd));
                            g *= bright;
                        }
                    }
                    acc / 3.0
                }
                _ => {
                    let mut acc = 0.0;
                    let mut g = 1.0;
                    for (ratio, amp) in &parts {
                        acc += amp * g * libm::sin(TAU * ph * ratio);
                        g *= bright;
                    }
                    acc
                }
            };
            l[s] = sample * el;
            r[s] = sample * er;
        }
        [l, r]
    });
    let mut mix = [vec![0.0; n], vec![0.0; n]];
    for o in out {
        for c in 0..2 {
            for (m, x) in mix[c].iter_mut().zip(&o[c]) {
                *m += x;
            }
        }
    }
    mix
}

/// A drone played by a synth: each voice is one long note of the synth,
/// held the whole piece, gliding to its next chord tone with a progression.
/// Its loudness follows the field as for any drone; the patch also sees it
/// as `note.level`, to open a filter with it, say.
#[allow(clippy::too_many_arguments)]
fn drone_synth(field: &Field, seed: u64, voices: usize, synth: &Synth, scale: &Scale, key: f64, harmony: Option<&Harmony>, n: usize, duration: f64) -> [Vec<f64>; 2] {
    let env = drone_envelopes(field, seed, voices, duration);
    let plan = harmony.map(|h| h.voicing(voices, key));
    let glide = 1.0 - libm::exp(-1.0 / (0.06 * SR));
    let out: Vec<[Vec<f64>; 2]> = crate::media::par_map(voices, |v| {
        let target = |pos: f64| match &plan {
            Some(plan) => key * libm::pow(2.0, crate::harmony::pitch_at(&plan[v], pos) / 12.0),
            None => scale.freq(key, drone_degree(scale, v)),
        };
        let mut freq = target(0.0);
        let mut voice = synth.voice(seed ^ (v as u64 + 1).wrapping_mul(0x51ED_270B));
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        for s in 0..n {
            let pos = s as f64 / n as f64;
            freq += (target(pos) - freq) * glide;
            let (nl, nr) = (at(&env[v][0], pos), at(&env[v][1], pos));
            let e = 0.5 * (nl + nr);
            let (gl, gr) = pan(0.5 + 0.35 * (nr - nl));
            let ctx = NoteCtx { freq, velocity: 1.0, gate: true, time: s as f64 / SR, level: e };
            let sample = voice.tick(&ctx);
            l[s] = sample * e * gl * std::f64::consts::SQRT_2;
            r[s] = sample * e * gr * std::f64::consts::SQRT_2;
        }
        crate::synth::declick(&mut l);
        crate::synth::declick(&mut r);
        [l, r]
    });
    let mut mix = [vec![0.0; n], vec![0.0; n]];
    for o in out {
        for c in 0..2 {
            for (m, x) in mix[c].iter_mut().zip(&o[c]) {
                *m += x;
            }
        }
    }
    mix
}

/// One struck note, in mono: `freq` Hz, ringing about `d` seconds.
fn strike(kind: Strike, freq: f64, d: f64, noise: &mut Noise) -> Vec<f64> {
    let len = ((d * SR) as usize).max(16);
    let mut out = vec![0.0; len];
    match kind {
        Strike::Pluck => {
            // Karplus–Strong: a burst of noise circulating in a delay the length
            // of one period, softened at each pass, like a string.
            let p = ((SR / freq) as usize).max(2);
            let mut line: Vec<f64> = (0..p).map(|_| noise.next()).collect();
            let g = libm::exp(libm::log(0.001) / (d * freq));
            let mut prev = 0.0;
            for (k, o) in out.iter_mut().enumerate() {
                let i = k % p;
                let y = line[i];
                *o = y;
                line[i] = g * 0.5 * (y + prev);
                prev = y;
            }
        }
        Strike::Drum => {
            for (k, o) in out.iter_mut().enumerate() {
                let tt = k as f64 / SR;
                let f = freq * (1.0 + 1.5 * libm::exp(-tt * 30.0));
                let body = libm::sin(TAU * f * tt) * libm::exp(-tt * 6.0 / d);
                let skin = noise.next() * libm::exp(-tt * 60.0) * 0.4;
                *o = body + skin;
            }
        }
        Strike::Tick => {
            let mut hp = Svf::default();
            for (k, o) in out.iter_mut().enumerate() {
                let tt = k as f64 / SR;
                let (low, _) = hp.tick(noise.next(), 2500.0, 0.7);
                *o = (noise.next() - low) * libm::exp(-tt * 180.0);
            }
        }
        _ => {
            // Modal instruments: a few partials, the high ones dying first.
            let modes: &[(f64, f64, f64)] = match kind {
                Strike::Bell => &[(1.0, 1.0, 1.0), (2.76, 0.3, 1.6), (5.4, 0.12, 2.6)],
                Strike::Marimba => &[(1.0, 1.0, 1.0), (3.93, 0.35, 3.0), (9.2, 0.12, 6.0)],
                Strike::Kalimba => &[(1.0, 1.0, 1.0), (5.4, 0.28, 4.0), (11.8, 0.1, 7.0)],
                Strike::Glass => &[(1.0, 1.0, 1.0), (2.76, 0.5, 1.2), (5.4, 0.3, 1.5), (8.93, 0.15, 2.0)],
                _ => &[(1.0, 1.0, 1.0), (2.7, 0.5, 2.0)], // wood
            };
            for (k, o) in out.iter_mut().enumerate() {
                let tt = k as f64 / SR;
                let attack = (tt / 0.002).min(1.0);
                let mut acc = 0.0;
                for (ratio, amp, fast) in modes {
                    acc += amp * libm::sin(TAU * freq * ratio * tt) * libm::exp(-tt * 5.0 * fast / d);
                }
                let click = if matches!(kind, Strike::Wood | Strike::Marimba) { noise.next() * libm::exp(-tt * 400.0) * 0.3 } else { 0.0 };
                *o = attack * acc + click;
            }
        }
    }
    out
}

/// Where an event of a `notes` layer falls: when (seconds), which degree of
/// the scale, at what frequency, ringing how long.
pub struct Placed {
    pub when: f64,
    pub degree: usize,
    pub freq: f64,
    pub ring: f64,
    /// Which event of the pattern it is.
    pub index: usize,
    /// Its loudness, relative to the event's intensity.
    pub vel: f64,
    /// For an arpeggio: how long it is held (seconds), from its gate.
    pub hold: Option<f64>,
}

pub fn place_note(e: &Event, kind: Strike, scale: &Scale, key: f64, tempo: f64, duration: f64) -> Placed {
    // Three octaves of the scale, from the bottom of the canvas to the top.
    let degrees = 3 * scale.len();
    let mut when = e.t.clamp(0.0, 1.0) * duration;
    if tempo > 0.0 {
        // on the grid: sixteenth notes of the tempo
        let grid = 60.0 / tempo / 4.0;
        when = (when / grid).round() * grid;
    }
    let d = (1.0 - e.y.clamp(0.0, 1.0)) * degrees as f64;
    let degree = (d as usize).min(degrees - 1);
    let freq = scale.freq(key, degree) * if kind == Strike::Drum { 0.25 } else { 1.0 };
    let ring = (0.08 + e.size * 20.0).min(3.0) * match kind {
        Strike::Tick => 0.15,
        Strike::Wood => 0.4,
        Strike::Drum => 0.8,
        Strike::Glass | Strike::Bell => 1.6,
        _ => 1.0,
    };
    Placed { when, degree, freq, ring, index: 0, vel: 1.0, hold: None }
}

/// Every event of a `notes` layer, placed: on the scale (by height), or on
/// the chord of the moment (`harmony`), in an arpeggio (`arp`), swung and
/// loosened. Without any of these, exactly `place_note` for each event.
pub fn placements(events: &[Event], kind: Strike, scale: &Scale, key: f64, tempo: f64, duration: f64, feel: &Feel, seed: u64) -> Vec<Placed> {
    let mut out: Vec<Placed> = events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut p = place_note(e, kind, scale, key, tempo, duration);
            p.index = i;
            p
        })
        .collect();
    if feel.plain() {
        return out;
    }
    // the events in time order (an arpeggio counts its notes that way)
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by(|a, b| events[*a].t.total_cmp(&events[*b].t).then(a.cmp(b)));
    let mut rank = vec![0usize; events.len()];
    for (r, i) in order.iter().enumerate() {
        rank[*i] = r;
    }
    if let Some(h) = &feel.harmony {
        let octaves = feel.arp.map_or(3, |a| a.octaves);
        for p in out.iter_mut() {
            let e = &events[p.index];
            let ladder = h.ladder(e.t.clamp(0.0, 1.0), key, octaves);
            let k = match feel.arp {
                Some(a) => a.mode.index(rank[p.index], ladder.len(), seed),
                None => (((1.0 - e.y.clamp(0.0, 1.0)) * ladder.len() as f64) as usize).min(ladder.len() - 1),
            };
            p.degree = k;
            p.freq = key * libm::pow(2.0, ladder[k] / 12.0) * if kind == Strike::Drum { 0.25 } else { 1.0 };
        }
    }
    if tempo > 0.0 && feel.swing != 0.5 {
        let grid = 60.0 / tempo / 4.0;
        for p in out.iter_mut() {
            let n = (p.when / grid).round() as i64;
            if n % 2 == 1 {
                p.when += (feel.swing - 0.5) * 2.0 * grid;
            }
        }
    }
    if feel.humanize > 0.0 {
        for p in out.iter_mut() {
            let u = |salt: u64| (crate::eval::hash(&[seed, 0x4D4E, p.index as u64, salt]) >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
            p.when = (p.when + u(0) * feel.humanize * 0.03).max(0.0);
            p.vel = (1.0 + u(1) * feel.humanize * 0.4).max(0.05);
        }
    }
    if let Some(a) = feel.arp {
        for (r, i) in order.iter().enumerate() {
            let next = order.get(r + 1).map(|j| out[*j].when);
            let p = &mut out[*i];
            let gap = next.map_or(p.ring, |w| (w - p.when).max(0.02));
            p.hold = Some((gap * a.gate).max(0.02));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn notes(events: &[Event], kind: Strike, scale: &Scale, key: f64, tempo: f64, seed: u64, n: usize, duration: f64, feel: &Feel, synth: Option<&Synth>) -> [Vec<f64>; 2] {
    let mut l = vec![0.0; n];
    let mut r = vec![0.0; n];
    for p in placements(events, kind, scale, key, tempo, duration, feel, seed) {
        let (i, e) = (p.index, &events[p.index]);
        let onset = (p.when * SR) as usize;
        if onset >= n {
            continue;
        }
        let (freq, ring) = (p.freq, p.ring);
        let note_seed = seed ^ (i as u64 + 1).wrapping_mul(0x9E37_79B9);
        let tone = match synth {
            // held for its arpeggio gate, or as long as the event rings; its
            // velocity (for the patch to use) from the event's intensity
            Some(sy) => sy.note(freq, (e.intensity * p.vel).clamp(0.0, 1.0), p.hold.unwrap_or(ring), note_seed),
            None => strike(kind, freq, ring, &mut Noise(note_seed)),
        };
        let amp = if p.vel == 1.0 { e.intensity.clamp(0.0, 4.0) * 0.25 } else { e.intensity.clamp(0.0, 4.0) * 0.25 * p.vel };
        let (gl, gr) = pan(e.x);
        for (k, s) in tone.iter().enumerate() {
            let j = onset + k;
            if j >= n {
                break;
            }
            l[j] += s * amp * gl;
            r[j] += s * amp * gr;
        }
    }
    [l, r]
}

fn texture(field: &Field, seed: u64, kind: Ambience, n: usize, duration: f64) -> [Vec<f64>; 2] {
    // How much of the canvas is lit, at control rate (100 Hz).
    let nk = (duration * 100.0) as usize + 2;
    let raw: Vec<f64> = crate::media::par_map(nk, |k| {
        let t = k as f64 / (nk - 1) as f64;
        let mut acc = 0.0;
        for j in 0..10 {
            for i in 0..10 {
                acc += field.eval([(i as f64 + 0.5) / 10.0, (j as f64 + 0.5) / 10.0, t], seed);
            }
        }
        acc / 100.0
    });
    let b = robust_bounds(&raw);
    let env: Vec<f64> = raw.iter().map(|x| crate::media::normalize(*x, b)).collect();

    let mut l = vec![0.0; n];
    let mut r = vec![0.0; n];
    let mut noise = Noise(seed ^ 0x7E57_u64);
    match kind {
        Ambience::Wind | Ambience::Waves => {
            let (mut fl, mut fr) = (Svf::default(), Svf::default());
            for s in 0..n {
                let tt = s as f64 / SR;
                let e = at(&env, s as f64 / n as f64);
                if kind == Ambience::Wind {
                    let cut = 180.0 + 1400.0 * e * e;
                    let (_, bl) = fl.tick(noise.next(), cut, 1.8);
                    let (_, br) = fr.tick(noise.next(), cut * 1.1, 1.8);
                    l[s] = bl * e;
                    r[s] = br * e;
                } else {
                    // waves: two slow swells, out of step between the ears
                    let swell = |p: f64| 0.15 + 0.85 * libm::pow(0.5 + 0.5 * libm::sin(TAU * tt / 7.3 + p), 3.0);
                    let (low_l, _) = fl.tick(noise.next(), 700.0, 0.8);
                    let (low_r, _) = fr.tick(noise.next(), 760.0, 0.8);
                    l[s] = low_l * swell(0.0) * (0.25 + 0.75 * e);
                    r[s] = low_r * swell(2.4) * (0.25 + 0.75 * e);
                }
            }
        }
        Ambience::Rain | Ambience::Crackle => {
            // grains: more of them where more is lit
            let rate = if kind == Ambience::Rain { 900.0 } else { 60.0 };
            for s in 0..n {
                let e = at(&env, s as f64 / n as f64);
                if noise.unit() >= rate * e / SR {
                    continue;
                }
                let (gl, gr) = pan(noise.unit());
                let amp = 0.3 + 0.7 * noise.unit();
                if kind == Ambience::Rain {
                    let f = 1800.0 + 4500.0 * noise.unit();
                    let len = (0.012 * SR) as usize;
                    for k in 0..len.min(n - s) {
                        let tt = k as f64 / SR;
                        let y = libm::sin(TAU * f * tt) * libm::exp(-tt * 350.0) * amp * 0.5;
                        l[s + k] += y * gl;
                        r[s + k] += y * gr;
                    }
                } else {
                    // a pop with a body: a short knock, resonating somewhere
                    // between 400 Hz and 2 kHz
                    let centre = 400.0 * libm::pow(5.0, noise.unit());
                    let len = ((0.004 + 0.012 * noise.unit()) * SR) as usize;
                    let mut f = Svf::default();
                    for k in 0..len.min(n - s) {
                        let decay = libm::exp(-(k as f64) / len as f64 * 5.0);
                        let (_, band) = f.tick(noise.next(), centre, 3.5);
                        let y = band * amp * decay * 2.0;
                        l[s + k] += y * gl;
                        r[s + k] += y * gr;
                    }
                }
            }
        }
    }
    [l, r]
}

// ───────────────────────────── the mix ─────────────────────────────

/// A ping-pong echo: what goes in on the left comes back on the right.
fn echo(buf: &mut [Vec<f64>; 2], amount: f64, delay: f64) {
    if amount <= 0.0 {
        return;
    }
    let d = ((delay * SR) as usize).max(1);
    let n = buf[0].len();
    let (mut dl, mut dr) = (vec![0.0; d], vec![0.0; d]);
    for s in 0..n {
        let i = s % d;
        let (el, er) = (dl[i], dr[i]);
        dl[i] = buf[0][s] * 0.5 + er * 0.45;
        dr[i] = buf[1][s] * 0.5 + el * 0.45;
        buf[0][s] += el * amount;
        buf[1][s] += er * amount;
    }
}

/// A room (a Freeverb: parallel combs, then allpasses), slightly different
/// on each side so the space is wide.
fn room(buf: &mut [Vec<f64>; 2], amount: f64) {
    if amount <= 0.0 {
        return;
    }
    const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
    let feedback = 0.7 + 0.28 * amount.min(1.0);
    let damp = 0.25;
    let n = buf[0].len();
    let input: Vec<f64> = (0..n).map(|s| (buf[0][s] + buf[1][s]) * 0.015).collect();
    for (c, spread) in [(0usize, 0usize), (1, 23)] {
        let mut wet = vec![0.0; n];
        for len in COMBS {
            let mut line = vec![0.0; len + spread];
            let mut store = 0.0;
            for s in 0..n {
                let i = s % line.len();
                let y = line[i];
                store = y * (1.0 - damp) + store * damp;
                line[i] = input[s] + store * feedback;
                wet[s] += y;
            }
        }
        for len in ALLPASSES {
            let mut line = vec![0.0; len + spread];
            for s in 0..n {
                let i = s % line.len();
                let b = line[i];
                line[i] = wet[s] + b * 0.5;
                wet[s] = b - wet[s];
            }
        }
        let w = amount.min(1.0) * 3.0;
        for s in 0..n {
            buf[c][s] += wet[s] * w;
        }
    }
}

/// Every layer, mixed, through the echo and the room, mastered.
pub fn render(layers: &[Arc<Layer>], mix: &Mix) -> [Vec<f64>; 2] {
    let n = (mix.duration * SR) as usize;
    let mut out = [vec![0.0; n], vec![0.0; n]];
    let mut tempo: f64 = 0.0;
    let mut dry = [vec![0.0; n], vec![0.0; n]];
    for layer in layers {
        if matches!(layer.as_ref(), Layer::Control { .. }) {
            continue;
        }
        if let Layer::Clip { input, gain } = layer.as_ref() {
            if let crate::input::Kind::Sound(clip) = &input.kind {
                // the original sound: not through the room nor the echo, at a
                // level comparable to the other layers
                let buf = clip.stereo(SR, n);
                let rms = (buf.iter().flat_map(|c| c.iter()).map(|x| x * x).sum::<f64>() / (2 * n).max(1) as f64).sqrt();
                let g = if rms > 1e-12 { 0.08 / rms * gain } else { 0.0 };
                for c in 0..2 {
                    for (o, x) in dry[c].iter_mut().zip(&buf[c]) {
                        *o += x * g;
                    }
                }
            }
            continue;
        }
        let (buf, gain) = match layer.as_ref() {
            Layer::Drone { field, seed, voices, scale, key, gain, harmony, synth: Some(sy), .. } => {
                (drone_synth(field, *seed, *voices, sy, scale, *key, harmony.as_deref(), n, mix.duration), *gain)
            }
            Layer::Drone { field, seed, voices, voice, scale, key, gain, harmony, synth: None } => match harmony {
                Some(h) => (drone_moving(field, *seed, *voices, *voice, *key, h, n, mix.duration), *gain),
                None => (drone(field, *seed, *voices, *voice, scale, *key, n, mix.duration), *gain),
            },
            Layer::Notes { events, strike, scale, key, gain, tempo: t, seed, feel, synth } => {
                if *t > 0.0 {
                    tempo = *t;
                }
                (notes(events, *strike, scale, *key, *t, *seed, n, mix.duration, feel, synth.as_deref()), *gain)
            }
            Layer::Texture { field, seed, kind, gain } => (texture(field, *seed, *kind, n, mix.duration), *gain),
            Layer::Clip { .. } | Layer::Control { .. } => unreachable!("handled above"),
        };
        // Each layer at a comparable level, then its own gain.
        let rms = (buf.iter().flat_map(|c| c.iter()).map(|x| x * x).sum::<f64>() / (2 * n).max(1) as f64).sqrt();
        let g = if rms > 1e-12 { 0.08 / rms * gain } else { 0.0 };
        for c in 0..2 {
            for (o, x) in out[c].iter_mut().zip(&buf[c]) {
                *o += x * g;
            }
        }
    }
    let delay = if tempo > 0.0 { 60.0 / tempo * 0.75 } else { 0.375 };
    echo(&mut out, mix.echo, delay);
    room(&mut out, mix.room);
    for c in 0..2 {
        for (o, x) in out[c].iter_mut().zip(&dry[c]) {
            *o += x;
        }
    }

    // Master: one level for both sides, gently limited, faded in and out.
    let peak = out.iter().flat_map(|c| c.iter()).fold(0.0f64, |m, x| m.max(x.abs())).max(1e-9);
    let fade = 0.5 * SR;
    for c in out.iter_mut() {
        for s in 0..n {
            let g = (s.min(n - 1 - s) as f64 / fade).min(1.0);
            c[s] = libm::tanh(1.5 * c[s] / peak) / libm::tanh(1.5) * 0.9 * g;
        }
    }
    out
}
