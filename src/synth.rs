//! Synths written in alea: `synth { … }` builds a patch of oscillators,
//! envelopes, filters and effects, played once per note.
//!
//! ```alea
//! let pad = synth {
//!   let env = adsr(attack: 0.4, decay: 0.3, sustain: 0.7, release: 1.5)
//!   lowpass(saw(note.freq) + saw(note.freq * 1.004), cutoff: 300 + 2500 * env) * env
//! }
//! render sound(notes(melody, instrument: pad))
//! ```
//!
//! A patch is a graph of signals ([`Sig`]). It is compiled once into a flat
//! program ([`Program`]): one operation per unit, in an order where each
//! reads only the ones before it; a unit used twice (a `let env = adsr(…)`
//! for both the filter and the loudness) is one unit, computed once. Each
//! note then runs the program sample after sample with its own state: the
//! phase of its oscillators, the memory of its filters, its envelope.
//!
//! Everything is computed with `libm` and deterministic noise, so a synth
//! gives the same samples on every platform and in the browser.

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::sync::Arc;

use crate::sound::SR;

/// What a synth knows about the note it plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteValue {
    /// Its frequency, in Hz.
    Freq,
    /// How hard it is played, about 0 to 1.
    Velocity,
    /// 1 while the note is held, 0 after.
    Gate,
    /// Seconds since the note started.
    Time,
    /// For a drone: how loud the field is there, 0 to 1 (1 for a note).
    Level,
}

pub const NOTE_VALUES: &[&str] = &["freq", "velocity", "gate", "time", "level"];

/// The names that mean units inside `synth { … }` (outside, `square` is a
/// shape, as before).
pub const UNITS: &[&str] = &[
    "sine", "saw", "square", "triangle", "pulse", "noise", "adsr", "perc", "lowpass", "highpass", "bandpass", "delay", "drive",
    "chorus",
];

impl NoteValue {
    pub fn parse(s: &str) -> Option<NoteValue> {
        Some(match s {
            "freq" | "frequency" => NoteValue::Freq,
            "velocity" => NoteValue::Velocity,
            "gate" => NoteValue::Gate,
            "time" => NoteValue::Time,
            "level" => NoteValue::Level,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Saw,
    Square,
    Triangle,
    Pulse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    Low,
    High,
    Band,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op2 {
    Add,
    Sub,
    Mul,
    Div,
}

/// A signal: one unit of a patch, and what it listens to.
#[derive(Debug)]
pub enum Sig {
    Const(f64),
    /// `note` itself, before `.freq` or another value is taken from it.
    NoteRecord,
    Note(NoteValue),
    Osc { wave: Wave, freq: Arc<Sig>, width: Arc<Sig> },
    Noise,
    /// Attack, decay, sustain level, release (seconds; the level 0–1).
    Adsr { attack: f64, decay: f64, sustain: f64, release: f64 },
    /// A struck envelope: up in `attack`, then down to silence in `release`, gate or not.
    Perc { attack: f64, release: f64 },
    Filter { pass: Pass, input: Arc<Sig>, cutoff: Arc<Sig>, resonance: Arc<Sig> },
    Delay { input: Arc<Sig>, time: f64, feedback: f64, mix: f64 },
    Drive { input: Arc<Sig>, amount: Arc<Sig> },
    Chorus { input: Arc<Sig>, depth: f64, rate: f64, mix: f64 },
    Bin(Op2, Arc<Sig>, Arc<Sig>),
    Neg(Arc<Sig>),
}

impl Sig {
    pub fn constant(&self) -> Option<f64> {
        if let Sig::Const(c) = self { Some(*c) } else { None }
    }
}

/// A synth: its patch, compiled, and a name (from `let name = synth { … }`).
#[derive(Debug)]
pub struct Synth {
    pub name: String,
    program: Program,
}

/// The longest a note may ring after its gate closes, whatever the patch says.
const MAX_TAIL: f64 = 12.0;

impl Synth {
    pub fn new(body: &Arc<Sig>) -> Result<Synth, String> {
        Ok(Synth { name: "synth".into(), program: Program::compile(body)? })
    }

    /// The same synth, named.
    pub fn named(&self, name: &str) -> Synth {
        Synth { name: name.to_string(), program: self.program.clone() }
    }

    /// How long a note held for `hold` seconds sounds: until its release
    /// envelopes and echoes have died away, or its struck envelopes have.
    pub fn length(&self, hold: f64) -> f64 {
        let p = &self.program;
        let mut len = hold + p.release + p.echo;
        if p.struck > 0.0 && !p.gated {
            // only struck envelopes: the note rings for them, not for its gate
            len = p.struck + p.echo;
        } else {
            len = len.max(p.struck + p.echo);
        }
        len.clamp(0.01, hold + MAX_TAIL)
    }

    /// A fresh voice: the program with its own state, its noise from `seed`.
    pub fn voice(&self, seed: u64) -> Voice<'_> {
        Voice::new(&self.program, seed)
    }

    /// One note, in mono: `freq` Hz, held `hold` seconds, at `velocity`.
    pub fn note(&self, freq: f64, velocity: f64, hold: f64, seed: u64) -> Vec<f64> {
        let len = ((self.length(hold) * SR) as usize).max(16);
        let release_at = (hold * SR) as usize;
        let mut v = self.voice(seed);
        let mut out = Vec::with_capacity(len);
        for k in 0..len {
            let ctx = NoteCtx { freq, velocity, gate: k < release_at, time: k as f64 / SR, level: 1.0 };
            out.push(v.tick(&ctx));
        }
        declick(&mut out);
        out
    }
}

/// A short fade at both ends, so a note never starts or stops with a click.
pub fn declick(out: &mut [f64]) {
    let n = out.len();
    let fade = ((0.002 * SR) as usize).min(n / 2);
    for k in 0..fade {
        let g = k as f64 / fade as f64;
        out[k] *= g;
        out[n - 1 - k] *= g;
    }
}

/// The note, as the voice sees it at one sample.
#[derive(Clone, Copy, Debug)]
pub struct NoteCtx {
    pub freq: f64,
    pub velocity: f64,
    pub gate: bool,
    pub time: f64,
    pub level: f64,
}

// ───────────────────────────── the program ─────────────────────────────

/// One operation: it writes slot `i` (its index in the program).
#[derive(Clone, Debug)]
enum Op {
    Const(f64),
    Note(NoteValue),
    Osc { wave: Wave, freq: usize, width: usize, state: usize },
    Noise,
    Adsr { attack: f64, decay: f64, sustain: f64, release: f64, state: usize },
    Perc { attack: f64, release: f64 },
    Filter { pass: Pass, input: usize, cutoff: usize, resonance: usize, state: usize },
    Delay { input: usize, len: usize, feedback: f64, mix: f64, state: usize },
    Drive { input: usize, amount: usize },
    Chorus { input: usize, depth: f64, rate: f64, mix: f64, state: usize },
    Bin(Op2, usize, usize),
    Neg(usize),
}

/// What an operation keeps between samples.
#[derive(Clone, Debug)]
enum State {
    Phase(f64),
    Env { released: Option<(f64, f64)>, level: f64 },
    Svf { ic1: f64, ic2: f64 },
    Line { buf: Vec<f64>, at: usize },
    Chorus { buf: Vec<f64>, at: usize, phase: f64 },
}

#[derive(Clone, Debug)]
struct Program {
    ops: Vec<Op>,
    states: Vec<State>,
    /// Is any envelope driven by the gate (adsr)?
    gated: bool,
    /// The longest adsr release, in seconds.
    release: f64,
    /// The longest struck envelope (perc: attack + release), in seconds.
    struck: f64,
    /// How long echoes go on after the sound stops.
    echo: f64,
}

impl Program {
    fn compile(root: &Arc<Sig>) -> Result<Program, String> {
        let mut p = Program { ops: Vec::new(), states: Vec::new(), gated: false, release: 0.0, struck: 0.0, echo: 0.0 };
        let mut seen: HashMap<*const Sig, usize> = HashMap::new();
        p.emit(root, &mut seen)?;
        Ok(p)
    }

    fn state(&mut self, s: State) -> usize {
        self.states.push(s);
        self.states.len() - 1
    }

    /// The slot of `s`, emitting it (after what it reads) the first time.
    fn emit(&mut self, s: &Arc<Sig>, seen: &mut HashMap<*const Sig, usize>) -> Result<usize, String> {
        if let Some(i) = seen.get(&Arc::as_ptr(s)) {
            return Ok(*i);
        }
        let op = match s.as_ref() {
            Sig::Const(c) => Op::Const(*c),
            Sig::NoteRecord => {
                return Err("`note` is the note being played: take a value from it, such as `note.freq` or `note.velocity`".into());
            }
            Sig::Note(v) => Op::Note(*v),
            Sig::Osc { wave, freq, width } => {
                let freq = self.emit(freq, seen)?;
                let width = self.emit(width, seen)?;
                let state = self.state(State::Phase(0.0));
                Op::Osc { wave: *wave, freq, width, state }
            }
            Sig::Noise => Op::Noise,
            Sig::Adsr { attack, decay, sustain, release } => {
                self.gated = true;
                self.release = self.release.max(*release);
                let state = self.state(State::Env { released: None, level: 0.0 });
                Op::Adsr { attack: *attack, decay: *decay, sustain: *sustain, release: *release, state }
            }
            Sig::Perc { attack, release } => {
                self.struck = self.struck.max(attack + release);
                Op::Perc { attack: *attack, release: *release }
            }
            Sig::Filter { pass, input, cutoff, resonance } => {
                let input = self.emit(input, seen)?;
                let cutoff = self.emit(cutoff, seen)?;
                let resonance = self.emit(resonance, seen)?;
                let state = self.state(State::Svf { ic1: 0.0, ic2: 0.0 });
                Op::Filter { pass: *pass, input, cutoff, resonance, state }
            }
            Sig::Delay { input, time, feedback, mix } => {
                let input = self.emit(input, seen)?;
                let len = ((time * SR) as usize).max(1);
                // until the echoes are 60 dB down
                let tail = if *feedback > 0.0 { time * libm::log(0.001) / libm::log(*feedback) } else { *time };
                self.echo = self.echo.max(tail.min(MAX_TAIL));
                let state = self.state(State::Line { buf: vec![0.0; len], at: 0 });
                Op::Delay { input, len, feedback: *feedback, mix: *mix, state }
            }
            Sig::Drive { input, amount } => {
                let input = self.emit(input, seen)?;
                let amount = self.emit(amount, seen)?;
                Op::Drive { input, amount }
            }
            Sig::Chorus { input, depth, rate, mix } => {
                let input = self.emit(input, seen)?;
                let state = self.state(State::Chorus { buf: vec![0.0; (0.05 * SR) as usize], at: 0, phase: 0.0 });
                Op::Chorus { input, depth: *depth, rate: *rate, mix: *mix, state }
            }
            Sig::Bin(op, a, b) => {
                let a = self.emit(a, seen)?;
                let b = self.emit(b, seen)?;
                Op::Bin(*op, a, b)
            }
            Sig::Neg(a) => Op::Neg(self.emit(a, seen)?),
        };
        self.ops.push(op);
        let i = self.ops.len() - 1;
        seen.insert(Arc::as_ptr(s), i);
        Ok(i)
    }
}

// ───────────────────────────── playing ─────────────────────────────

/// A program, playing one note: its slots and its state.
pub struct Voice<'a> {
    program: &'a Program,
    slots: Vec<f64>,
    states: Vec<State>,
    noise: u64,
    /// The last input and output of the voice's final high-pass (15 Hz):
    /// rumble no one hears, and any offset from zero, taken out.
    dc: (f64, f64),
}

impl<'a> Voice<'a> {
    fn new(program: &'a Program, seed: u64) -> Voice<'a> {
        Voice { program, slots: vec![0.0; program.ops.len()], states: program.states.clone(), noise: seed ^ 0x5EED_5717, dc: (0.0, 0.0) }
    }

    fn noise(&mut self) -> f64 {
        self.noise = self.noise.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.noise;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }

    /// The next sample.
    pub fn tick(&mut self, n: &NoteCtx) -> f64 {
        let program = self.program;
        for (i, op) in program.ops.iter().enumerate() {
            let s = &self.slots;
            let v = match op {
                Op::Const(c) => *c,
                Op::Note(v) => match v {
                    NoteValue::Freq => n.freq,
                    NoteValue::Velocity => n.velocity,
                    NoteValue::Gate => n.gate as u8 as f64,
                    NoteValue::Time => n.time,
                    NoteValue::Level => n.level,
                },
                Op::Osc { wave, freq, width, state } => {
                    let (f, w) = (s[*freq], s[*width]);
                    let State::Phase(ph) = &mut self.states[*state] else { unreachable!() };
                    let out = oscillate(*wave, *ph, f.abs() / SR, w);
                    *ph = (*ph + f / SR).rem_euclid(1.0);
                    out
                }
                Op::Noise => self.noise(),
                Op::Adsr { attack, decay, sustain, release, state } => {
                    let State::Env { released, level } = &mut self.states[*state] else { unreachable!() };
                    let out = if n.gate {
                        *released = None;
                        adsr_held(n.time, *attack, *decay, *sustain)
                    } else {
                        // released: from wherever it was, down to silence
                        let (from, at) = *released.get_or_insert((*level, n.time));
                        let u = ((n.time - at) / release.max(0.001)).min(1.0);
                        from * (1.0 - u) * (1.0 - u)
                    };
                    *level = out;
                    out
                }
                Op::Perc { attack, release } => perc(n.time, *attack, *release),
                Op::Filter { pass, input, cutoff, resonance, state } => {
                    let (x, fc, res) = (s[*input], s[*cutoff], s[*resonance]);
                    let State::Svf { ic1, ic2 } = &mut self.states[*state] else { unreachable!() };
                    // a state-variable filter (topology-preserving): stable at any setting
                    let g = libm::tan(PI * fc.clamp(10.0, 0.45 * SR) / SR);
                    let k = 2.0 - 1.95 * res.clamp(0.0, 1.0);
                    let a1 = 1.0 / (1.0 + g * (g + k));
                    let a2 = g * a1;
                    let a3 = g * a2;
                    let v3 = x - *ic2;
                    let v1 = a1 * *ic1 + a2 * v3;
                    let v2 = *ic2 + a2 * *ic1 + a3 * v3;
                    *ic1 = 2.0 * v1 - *ic1;
                    *ic2 = 2.0 * v2 - *ic2;
                    match pass {
                        Pass::Low => v2,
                        Pass::Band => v1,
                        Pass::High => x - k * v1 - v2,
                    }
                }
                Op::Delay { input, len, feedback, mix, state } => {
                    let x = s[*input];
                    let State::Line { buf, at } = &mut self.states[*state] else { unreachable!() };
                    let delayed = buf[*at];
                    buf[*at] = x + delayed * feedback;
                    *at = (*at + 1) % *len;
                    x + delayed * mix
                }
                Op::Drive { input, amount } => {
                    let gain = 1.0 + 20.0 * s[*amount].clamp(0.0, 1.0);
                    libm::tanh(s[*input] * gain) / libm::tanh(gain)
                }
                Op::Chorus { input, depth, rate, mix, state } => {
                    let x = s[*input];
                    let State::Chorus { buf, at, phase } = &mut self.states[*state] else { unreachable!() };
                    let n_buf = buf.len();
                    buf[*at] = x;
                    // a copy 7 to 25 ms late, the lateness swaying slowly
                    let late = (0.007 + 0.018 * depth.clamp(0.0, 1.0) * 0.5 * (1.0 + libm::sin(TAU * *phase))) * SR;
                    let pos = (*at as f64 - late).rem_euclid(n_buf as f64);
                    let (k, frac) = (pos as usize % n_buf, pos.fract());
                    let wet = buf[k] * (1.0 - frac) + buf[(k + 1) % n_buf] * frac;
                    *at = (*at + 1) % n_buf;
                    *phase = (*phase + rate / SR).rem_euclid(1.0);
                    x * (1.0 - 0.5 * mix) + wet * mix
                }
                Op::Bin(op, a, b) => {
                    let (a, b) = (s[*a], s[*b]);
                    match op {
                        Op2::Add => a + b,
                        Op2::Sub => a - b,
                        Op2::Mul => a * b,
                        Op2::Div => {
                            if b.abs() < 1e-12 { 0.0 } else { a / b }
                        }
                    }
                }
                Op::Neg(a) => -s[*a],
            };
            // a runaway patch is silenced, not passed on
            self.slots[i] = if v.is_finite() { v.clamp(-1e6, 1e6) } else { 0.0 };
        }
        let x = *self.slots.last().unwrap_or(&0.0);
        const R: f64 = 1.0 - TAU * 15.0 / SR;
        let y = x - self.dc.0 + R * self.dc.1;
        self.dc = (x, y);
        y.clamp(-8.0, 8.0)
    }
}

/// One sample of a waveform at `phase` (0 to 1), `dt` = frequency / rate.
/// Saw, square and pulse are corrected at their jumps (polyBLEP), so they
/// don't alias into whistles at high notes.
fn oscillate(wave: Wave, phase: f64, dt: f64, width: f64) -> f64 {
    match wave {
        Wave::Sine => libm::sin(TAU * phase),
        Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
        Wave::Saw => 2.0 * phase - 1.0 - blep(phase, dt),
        Wave::Square | Wave::Pulse => {
            let w = if wave == Wave::Square { 0.5 } else { width.clamp(0.02, 0.98) };
            let naive = if phase < w { 1.0 } else { -1.0 };
            naive + blep(phase, dt) - blep((phase - w).rem_euclid(1.0), dt)
        }
    }
}

fn blep(t: f64, dt: f64) -> f64 {
    if dt <= 0.0 {
        0.0
    } else if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// An adsr while the note is held: up to 1, down to the sustain level, and stay.
fn adsr_held(t: f64, attack: f64, decay: f64, sustain: f64) -> f64 {
    let attack = attack.max(0.001);
    if t < attack {
        t / attack
    } else {
        let u = ((t - attack) / decay.max(0.001)).min(1.0);
        sustain + (1.0 - sustain) * (1.0 - u) * (1.0 - u)
    }
}

/// A struck envelope: up in `attack`, then an exponential fall, silent at `attack + release`.
fn perc(t: f64, attack: f64, release: f64) -> f64 {
    let attack = attack.max(0.0005);
    if t < attack {
        t / attack
    } else {
        let u = (t - attack) / release.max(0.001);
        if u >= 1.0 { 0.0 } else { libm::exp(-6.9 * u) * (1.0 - u) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f64) -> Arc<Sig> {
        Arc::new(Sig::Const(x))
    }
    fn note(v: NoteValue) -> Arc<Sig> {
        Arc::new(Sig::Note(v))
    }
    fn osc(wave: Wave, freq: Arc<Sig>) -> Arc<Sig> {
        Arc::new(Sig::Osc { wave, freq, width: c(0.5) })
    }
    fn mul(a: Arc<Sig>, b: Arc<Sig>) -> Arc<Sig> {
        Arc::new(Sig::Bin(Op2::Mul, a, b))
    }
    fn render(s: &Arc<Sig>, freq: f64, hold: f64) -> Vec<f64> {
        Synth::new(s).unwrap().note(freq, 1.0, hold, 7)
    }
    /// The frequency of a signal, from its upward zero crossings.
    fn pitch(v: &[f64]) -> f64 {
        let ups: Vec<usize> = (1..v.len()).filter(|&k| v[k - 1] < 0.0 && v[k] >= 0.0).collect();
        (ups.len() - 1) as f64 / ((ups[ups.len() - 1] - ups[0]) as f64 / SR)
    }

    #[test]
    fn oscillators_play_the_note() {
        for wave in [Wave::Sine, Wave::Saw, Wave::Square, Wave::Triangle] {
            let v = render(&osc(wave, note(NoteValue::Freq)), 440.0, 0.5);
            let f = pitch(&v[2000..20000]);
            assert!((f - 440.0).abs() < 1.0, "{wave:?}: {f} Hz");
            let peak = v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
            assert!(peak > 0.9 && peak < 1.2, "{wave:?}: peak {peak}");
        }
    }

    #[test]
    fn the_note_lasts_as_long_as_its_envelope() {
        let env = Arc::new(Sig::Adsr { attack: 0.01, decay: 0.1, sustain: 0.5, release: 0.4 });
        let s = Synth::new(&mul(osc(Wave::Sine, c(220.0)), env)).unwrap();
        assert!((s.length(1.0) - 1.4).abs() < 1e-9, "held 1 s, released in 0.4 s");
        let v = s.note(220.0, 1.0, 1.0, 1);
        let level = |a: f64, b: f64| v[(a * SR) as usize..(b * SR) as usize].iter().fold(0.0f64, |m, x| m.max(x.abs()));
        assert!((level(0.5, 0.9) - 0.5).abs() < 0.02, "the sustain level while held: {}", level(0.5, 0.9));
        assert!(level(1.36, 1.395) < 0.01, "silent by the end of the release");
        assert!(level(1.1, 1.15) > level(1.3, 1.35), "and fading on the way");
        // a struck envelope rings for itself, however short the note
        let bell = Synth::new(&mul(osc(Wave::Sine, c(880.0)), Arc::new(Sig::Perc { attack: 0.002, release: 2.0 }))).unwrap();
        assert!((bell.length(0.1) - 2.002).abs() < 1e-9);
    }

    #[test]
    fn a_shared_unit_is_one_unit() {
        // the same envelope for the filter and the loudness: one adsr, one state
        let env = Arc::new(Sig::Adsr { attack: 0.1, decay: 0.1, sustain: 0.5, release: 0.2 });
        let cutoff = Arc::new(Sig::Bin(Op2::Add, c(200.0), mul(c(3000.0), env.clone())));
        let filt = Arc::new(Sig::Filter { pass: Pass::Low, input: osc(Wave::Saw, c(110.0)), cutoff, resonance: c(0.3) });
        let p = Program::compile(&mul(filt, env)).unwrap();
        assert_eq!(p.ops.iter().filter(|o| matches!(o, Op::Adsr { .. })).count(), 1);
    }

    #[test]
    fn a_lowpass_takes_the_highs_away() {
        let energy = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64;
        let through = |pass: Pass, fc: f64| {
            let f = Arc::new(Sig::Filter { pass, input: osc(Wave::Sine, note(NoteValue::Freq)), cutoff: c(fc), resonance: c(0.0) });
            energy(&render(&f, 4000.0, 0.3)[2000..])
        };
        let open = through(Pass::Low, 18000.0);
        assert!(through(Pass::Low, 400.0) < open * 0.02, "4 kHz through a 400 Hz low-pass");
        assert!(through(Pass::High, 400.0) > open * 0.8, "and through a 400 Hz high-pass");
    }

    #[test]
    fn the_same_every_time() {
        let s = Arc::new(Sig::Bin(Op2::Add, osc(Wave::Saw, note(NoteValue::Freq)), mul(Arc::new(Sig::Noise), c(0.2))));
        let s = Arc::new(Sig::Delay { input: s, time: 0.1, feedback: 0.5, mix: 0.4 });
        assert_eq!(render(&s, 330.0, 0.3), render(&s, 330.0, 0.3));
    }

    #[test]
    fn a_runaway_patch_is_silenced() {
        let s = Arc::new(Sig::Bin(Op2::Div, c(1.0), c(0.0)));
        assert!(render(&s, 220.0, 0.1).iter().all(|x| x.is_finite()));
        let loud = mul(osc(Wave::Saw, c(100.0)), c(1e9));
        assert!(render(&loud, 220.0, 0.1).iter().all(|x| x.abs() <= 8.0));
    }

    #[test]
    fn no_offset_and_no_rumble() {
        // a constant (or a lopsided wave) doesn't push the speaker one way
        let v = render(&c(0.5), 220.0, 1.0);
        let tail = &v[(0.5 * SR) as usize..(0.9 * SR) as usize];
        assert!(tail.iter().all(|x| x.abs() < 0.01), "a constant fades to 0");
        let pulse = Arc::new(Sig::Osc { wave: Wave::Pulse, freq: c(110.0), width: c(0.1) });
        let v = render(&pulse, 110.0, 1.0);
        let mean = v[(0.3 * SR) as usize..(0.9 * SR) as usize].iter().sum::<f64>() / (0.6 * SR);
        assert!(mean.abs() < 0.01, "a 10 % pulse, centred: {mean}");
    }

    #[test]
    fn note_alone_is_not_a_sound() {
        assert!(Synth::new(&Arc::new(Sig::NoteRecord)).unwrap_err().contains("note.freq"));
    }

    /// Through the language: a work's source, its first sound render, played.
    mod language {
        use crate::synth::SR;

        fn edition(src: &str, seed: u64) -> Result<crate::eval::Edition, String> {
            static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let k = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!("alea-synth-test-{}-{k}.alea", std::process::id()));
            std::fs::write(&path, src).unwrap();
            let m = crate::library::load(path.to_str().unwrap()).map_err(|e| e.msg.clone())?;
            let ed = crate::eval::resolve(&m, seed).map_err(|e| e.msg.clone());
            let _ = std::fs::remove_file(&path);
            ed
        }

        fn sound(src: &str, seed: u64) -> Result<Vec<Vec<f64>>, String> {
            let ed = edition(src, seed)?;
            let r = ed.renders.iter().find(|r| r.kind == "sound").expect("a sound render");
            crate::media::sound_channels(r, seed).map_err(|e| e.msg.clone())
        }

        const WORK: &str = r#"
work T {
  let pad = synth {
    let env = adsr(attack: 0.05, release: 0.3)
    lowpass(saw() + saw(note.freq * ~around(1.0, 0.01)), cutoff: 400 + 3000 * env) * env
  }
  render sound(notes(steps("x.x.x.x."), instrument: pad, tempo: 120), duration: 3)
}"#;

        #[test]
        fn a_synth_plays_its_notes() {
            let s = sound(WORK, 1).unwrap();
            let rms = (s[0].iter().map(|x| x * x).sum::<f64>() / s[0].len() as f64).sqrt();
            assert!(rms > 0.05, "it sounds: rms {rms}");
            assert_eq!(s[0].len(), (3.0 * SR) as usize);
        }

        #[test]
        fn an_edition_is_its_seed() {
            assert_eq!(sound(WORK, 1).unwrap(), sound(WORK, 1).unwrap(), "the same seed, the same samples");
            assert_ne!(sound(WORK, 1).unwrap(), sound(WORK, 2).unwrap(), "another seed, another detune");
        }

        #[test]
        fn square_is_a_shape_outside_and_an_oscillator_inside() {
            let src = r#"
work T {
  let s = synth { square() * perc(release: 0.2) }
  render image(square(), width: 64)
  render sound(notes(steps("x..."), instrument: s), duration: 1)
}"#;
            let ed = edition(src, 1).unwrap();
            assert_eq!(ed.renders.len(), 2);
            assert!(sound(src, 1).unwrap()[0].iter().any(|x| x.abs() > 0.01));
        }

        #[test]
        fn a_synth_is_named_after_its_let() {
            let ed = edition(WORK, 1).unwrap();
            let r = ed.renders.iter().find(|r| r.kind == "sound").unwrap();
            let layers: Vec<_> = r
                .positional
                .iter()
                .filter_map(|(v, _)| if let crate::eval::Val::Sound(l) = v { Some(l.clone()) } else { None })
                .collect();
            let score = crate::midi::score(&layers, 3.0, None, "t");
            assert!(score.tracks.iter().any(|t| t == "notes - pad"), "{:?}", score.tracks);
        }

        #[test]
        fn instruments_are_works() {
            // a work whose output is a synth, used by another, with a parameter;
            // a function inside a synth block
            let dir = std::env::temp_dir().join(format!("alea-synth-lib-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("organ.alea"),
                "work Organ(stops = 3) {\n  output synth {\n    let pipe(k) = sine(note.freq * k) / k\n    pipe(1) + pipe(2) * (stops > 1) + pipe(4) * (stops > 2)\n  }\n}\n",
            )
            .unwrap();
            let main = dir.join("main.alea");
            std::fs::write(
                &main,
                "use \"organ.alea\"\nwork T {\n  render sound(drone(clouds(), voices: 2, instrument: Organ(stops: 2)), duration: 1)\n}\n",
            )
            .unwrap();
            let m = crate::library::load(main.to_str().unwrap()).unwrap();
            let ed = crate::eval::resolve(&m, 1).unwrap();
            let r = &ed.renders[0];
            let s = crate::media::sound_channels(r, 1).unwrap();
            assert!(s[0].iter().any(|x| x.abs() > 0.01), "the organ plays");
            let layers: Vec<_> = r
                .positional
                .iter()
                .filter_map(|(v, _)| if let crate::eval::Val::Sound(l) = v { Some(l.clone()) } else { None })
                .collect();
            let score = crate::midi::score(&layers, 1.0, None, "t");
            assert_eq!(score.tracks, vec!["drone - organ"], "named after its work");
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn mistakes_say_how_to_fix_them() {
            let err = |body: &str| {
                let src = format!("work T {{\n  {body}\n  render image(circle())\n}}");
                edition(&src, 1).err().unwrap_or_default()
            };
            assert!(err("let s = synth { sine(note.pitch) }").contains("freq, velocity, gate, time, level"));
            assert!(err("let s = synth { sine() * x }").contains("not pictures"));
            assert!(err("let s = synth { lowpass(cutoff: 400) }").contains("needs its `input`"));
            assert!(err("let s = synth { sine() * adsr(release: note.velocity) }").contains("set once for the whole synth"));
            assert!(err("let s = synth { note }").contains("note.freq"));
            assert!(err("let s = synth { sine() }\n  let c = circle() * s").contains("is an instrument"));
            assert!(err("let s = synth { }").contains("ends with its sound"));
            assert!(err("let s = synth { sine() }\n  let mel = notes(steps(\"x\"), instrument: \"piano\")").contains("or a synth"));
        }
    }
}
