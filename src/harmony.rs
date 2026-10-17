//! Harmony: note names, meters, chord progressions and how voices move
//! through them.
//!
//! A progression (`chords("i iv V7 i", key: "A3", bars: 8)`) is a list of
//! chords over the work, each a root (in semitones above the key) and its
//! intervals. It can be written with roman numerals (relative to the scale),
//! with chord names ("Am F C G"), or drawn: `"functional"` walks between
//! tonic, subdominant and dominant chords the way tonal music does, with a
//! cadence at the end of each phrase. Layers given `harmony:` follow it:
//! `notes` pick chord tones, a `drone`'s voices move to the nearest chord tone
//! at each change, and `arp` plays the chord one note at a time.

use rand::Rng;

/// A note name as a frequency: "A4" is 440 Hz; "C#3", "Bb2", "e5"…
pub fn note_hz(s: &str) -> Option<f64> {
    let s = s.trim();
    let mut chars = s.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let base = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    } as i32;
    let rest: String = chars.collect();
    let (acc, octave) = match rest.chars().next() {
        Some('#') | Some('♯') => (1, &rest[rest.chars().next().unwrap().len_utf8()..]),
        Some('b') | Some('♭') => (-1, &rest[rest.chars().next().unwrap().len_utf8()..]),
        _ => (0, rest.as_str()),
    };
    let octave: i32 = octave.parse().ok()?;
    if !(-1..=9).contains(&octave) {
        return None;
    }
    let midi = (octave + 1) * 12 + base + acc;
    Some(440.0 * libm::pow(2.0, (midi as f64 - 69.0) / 12.0))
}

/// A frequency as a (fractional) MIDI note number.
pub fn semitones(hz: f64) -> f64 {
    69.0 + 12.0 * libm::log2(hz / 440.0)
}

/// `meter(tempo, bars, beats)`: the length of a piece in bars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meter {
    pub tempo: f64,
    pub bars: f64,
    pub beats: f64,
}

impl Meter {
    pub fn beat(&self) -> f64 {
        60.0 / self.tempo
    }
    pub fn bar(&self) -> f64 {
        self.beat() * self.beats
    }
    pub fn length(&self) -> f64 {
        self.bar() * self.bars
    }
}

#[derive(Clone, Debug)]
pub struct Chord {
    /// Where it starts and ends, in the work's time (0 to 1).
    pub start: f64,
    pub end: f64,
    /// Semitones above the progression's key (0 to 11).
    pub root: f64,
    /// Semitones above the root, from 0, ascending.
    pub intervals: Vec<f64>,
    pub name: String,
}

#[derive(Debug)]
pub struct Harmony {
    /// The key, in Hz: the chords' roots are counted from it.
    pub key: f64,
    pub chords: Vec<Chord>,
    /// The tempo it was laid out with (from a meter), if any.
    pub tempo: Option<f64>,
}

impl Harmony {
    /// The chord at time t.
    pub fn at(&self, t: f64) -> &Chord {
        let k = self.chords.iter().position(|c| t < c.end).unwrap_or(self.chords.len() - 1);
        &self.chords[k]
    }

    /// How far above `layer_key` the progression's key sits, within an
    /// octave: a layer keeps its own register, the progression gives the notes.
    fn offset(&self, layer_key: f64) -> f64 {
        (semitones(self.key) - semitones(layer_key)).rem_euclid(12.0)
    }

    /// The tones of the chord at t, ascending over `octaves` octaves, in
    /// semitones above `layer_key`.
    pub fn ladder(&self, t: f64, layer_key: f64, octaves: usize) -> Vec<f64> {
        let c = self.at(t);
        let base = self.offset(layer_key) + c.root;
        let mut out = Vec::new();
        for o in 0..octaves.max(1) {
            for i in &c.intervals {
                out.push(base + i + 12.0 * o as f64);
            }
        }
        out.sort_by(|a, b| a.total_cmp(b));
        out.dedup();
        out
    }

    /// How `voices` voices move through the progression, each to the
    /// nearest free chord tone at each change (voice leading): for each
    /// voice, the moments it changes and its pitch, in semitones above
    /// `layer_key`.
    pub fn voicing(&self, voices: usize, layer_key: f64) -> Vec<Vec<(f64, f64)>> {
        let first = self.ladder(0.0, layer_key, 4);
        let mut now: Vec<f64> = (0..voices).map(|v| first[v.min(first.len() - 1)] + 12.0 * (v / first.len().max(1)) as f64).collect();
        let mut out: Vec<Vec<(f64, f64)>> = now.iter().map(|p| vec![(0.0, *p)]).collect();
        for c in self.chords.iter().skip(1) {
            let tones = self.ladder(c.start, layer_key, 6);
            let classes: Vec<f64> = tones.iter().map(|p| p.rem_euclid(12.0)).collect();
            let mut order: Vec<usize> = (0..voices).collect();
            order.sort_by(|a, b| now[*a].total_cmp(&now[*b]));
            let mut taken: Vec<f64> = Vec::new();
            for v in order {
                // candidates: every pitch of a chord tone within an octave of the voice
                let mut best: Option<(f64, f64)> = None;
                for pc in &classes {
                    for o in -2..=2 {
                        let p = (now[v] - now[v].rem_euclid(12.0)) + pc + 12.0 * o as f64;
                        let d = (p - now[v]).abs();
                        // a pitch class another voice already holds counts as farther
                        let penalty = if taken.iter().any(|q| (q - p).abs() < 1e-9) {
                            24.0
                        } else if taken.iter().any(|q| (q.rem_euclid(12.0) - pc).abs() < 1e-9) && taken.len() < classes.len() {
                            1.5
                        } else {
                            0.0
                        };
                        if best.is_none_or(|(_, s)| d + penalty < s) {
                            best = Some((p, d + penalty));
                        }
                    }
                }
                let p = best.map_or(now[v], |b| b.0);
                taken.push(p);
                if (p - now[v]).abs() > 1e-9 {
                    out[v].push((c.start, p));
                    now[v] = p;
                }
            }
        }
        out
    }
}

/// The pitch at time t of a voice's schedule.
pub fn pitch_at(schedule: &[(f64, f64)], t: f64) -> f64 {
    let mut p = schedule[0].1;
    for (start, q) in schedule {
        if *start <= t {
            p = *q;
        } else {
            break;
        }
    }
    p
}

// ───────────────────────────── reading chords ─────────────────────────────

/// Seven steps to count roman numerals on: the scale itself if it has seven
/// notes, otherwise the major or minor scale it sounds closest to.
pub fn seven(steps: &[f64]) -> Vec<f64> {
    if steps.len() == 7 {
        return steps.to_vec();
    }
    if steps.iter().any(|s| (*s - 3.0).abs() < 1e-9) {
        vec![0., 2., 3., 5., 7., 8., 10.]
    } else {
        vec![0., 2., 4., 5., 7., 9., 11.]
    }
}

/// The triad (or seventh chord) built on degree `d` of a seven-note scale,
/// stacking its own thirds: its intervals above the root.
fn diatonic(steps: &[f64], d: usize, seventh: bool) -> Vec<f64> {
    let at = |k: usize| steps[k % 7] + 12.0 * (k / 7) as f64;
    let root = at(d);
    let mut v = vec![0.0, at(d + 2) - root, at(d + 4) - root];
    if seventh {
        v.push(at(d + 6) - root);
    }
    v
}

fn quality(intervals: &[f64]) -> &'static str {
    let has = |x: f64| intervals.iter().any(|i| (i - x).abs() < 1e-9);
    match (has(3.0), has(4.0), has(6.0), has(8.0)) {
        (true, _, true, _) => "dim",
        (_, true, _, true) => "aug",
        (true, _, _, _) => "minor",
        _ => "major",
    }
}

const ROMAN: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

/// The roman numeral of a chord, for its name.
fn roman_name(d: usize, intervals: &[f64]) -> String {
    let q = quality(intervals);
    let mut s = if q == "minor" || q == "dim" { ROMAN[d].to_lowercase() } else { ROMAN[d].to_string() };
    match q {
        "dim" => s.push('°'),
        "aug" => s.push('+'),
        _ => {}
    }
    if intervals.len() > 3 {
        s.push('7');
    }
    s
}

/// One chord of a written progression: its root above the key, its
/// intervals, and its weight (how long it lasts, relatively).
fn read_chord(token: &str, steps: &[f64], key_pc: f64) -> Result<(f64, Vec<f64>, f64, String), String> {
    let (body, weight) = match token.split_once(':') {
        Some((b, w)) => (b, w.parse::<f64>().map_err(|_| format!("`{token}`: after `:` comes how long it lasts, a number"))?),
        None => (token, 1.0),
    };
    if !(weight > 0.0) {
        return Err(format!("`{token}`: a chord lasts more than nothing"));
    }
    let mut rest = body;
    // a chord name: a letter A–G
    if let Some(first) = rest.chars().next().filter(|c| ('A'..='G').contains(c)) {
        let mut pc = match first {
            'C' => 0.0,
            'D' => 2.0,
            'E' => 4.0,
            'F' => 5.0,
            'G' => 7.0,
            'A' => 9.0,
            _ => 11.0,
        };
        rest = &rest[1..];
        if let Some(r) = rest.strip_prefix('#') {
            pc += 1.0;
            rest = r;
        } else if let Some(r) = rest.strip_prefix('b') {
            pc -= 1.0;
            rest = r;
        }
        let intervals: Vec<f64> = match rest {
            "" | "M" | "maj" => vec![0., 4., 7.],
            "m" | "min" | "-" => vec![0., 3., 7.],
            "dim" | "°" | "o" => vec![0., 3., 6.],
            "aug" | "+" => vec![0., 4., 8.],
            "7" => vec![0., 4., 7., 10.],
            "maj7" | "M7" | "Δ" => vec![0., 4., 7., 11.],
            "m7" | "min7" | "-7" => vec![0., 3., 7., 10.],
            "m7b5" | "ø" => vec![0., 3., 6., 10.],
            "dim7" | "°7" | "o7" => vec![0., 3., 6., 9.],
            "sus2" => vec![0., 2., 7.],
            "sus4" | "sus" => vec![0., 5., 7.],
            "9" => vec![0., 4., 7., 10., 14.],
            "m9" => vec![0., 3., 7., 10., 14.],
            "add9" => vec![0., 4., 7., 14.],
            "6" => vec![0., 4., 7., 9.],
            "m6" => vec![0., 3., 7., 9.],
            other => return Err(format!("`{body}`: `{other}` is not a chord quality alea knows (m, dim, aug, 7, maj7, m7, m7b5, dim7, sus2, sus4, 6, m6, 9, m9, add9)")),
        };
        return Ok(((pc - key_pc).rem_euclid(12.0), intervals, weight, body.to_string()));
    }
    // a roman numeral, on the scale
    let mut shift = 0.0;
    loop {
        if let Some(r) = rest.strip_prefix('b').or_else(|| rest.strip_prefix('♭')) {
            shift -= 1.0;
            rest = r;
        } else if let Some(r) = rest.strip_prefix('#').or_else(|| rest.strip_prefix('♯')) {
            shift += 1.0;
            rest = r;
        } else {
            break;
        }
    }
    let numeral_len = rest.chars().take_while(|c| matches!(c, 'I' | 'V' | 'i' | 'v')).count();
    if numeral_len == 0 {
        return Err(format!("`{body}` is not a chord: write roman numerals (I ii iii IV V vi vii°) or chord names (Am F C G7)"));
    }
    let numeral = &rest[..numeral_len];
    let d = ROMAN.iter().position(|r| r.eq_ignore_ascii_case(numeral)).ok_or_else(|| format!("`{numeral}` is not a roman numeral from I to VII"))?;
    let lower = numeral.chars().next().unwrap().is_lowercase();
    let suffix = &rest[numeral_len..];
    let seven = seven(steps);
    let root = (seven[d] + shift).rem_euclid(12.0);
    let third = if lower { 3.0 } else { 4.0 };
    let mut intervals: Vec<f64> = vec![0.0, third, 7.0];
    let mut s = suffix;
    let take = |p: &str, s: &mut &str| -> bool {
        match s.strip_prefix(p) {
            Some(r) => {
                *s = r;
                true
            }
            None => false,
        }
    };
    if take("°", &mut s) || take("o", &mut s) || take("dim", &mut s) {
        intervals = vec![0.0, 3.0, 6.0];
        if take("7", &mut s) {
            intervals.push(9.0);
        }
    } else if take("ø", &mut s) {
        intervals = vec![0.0, 3.0, 6.0, 10.0];
        take("7", &mut s);
    } else if take("+", &mut s) || take("aug", &mut s) {
        intervals = vec![0.0, 4.0, 8.0];
    }
    if take("maj7", &mut s) || take("M7", &mut s) {
        intervals.push(11.0);
    } else if take("7", &mut s) {
        intervals.push(10.0);
    } else if take("9", &mut s) {
        intervals.push(10.0);
        intervals.push(14.0);
    } else if take("6", &mut s) {
        intervals.push(9.0);
    }
    if take("sus4", &mut s) || take("sus", &mut s) {
        intervals[1] = 5.0;
    } else if take("sus2", &mut s) {
        intervals[1] = 2.0;
    }
    if !s.is_empty() {
        return Err(format!("`{body}`: `{s}` is not something alea reads after a numeral (°, ø, +, 7, maj7, 9, 6, sus2, sus4)"));
    }
    intervals.sort_by(|a, b| a.total_cmp(b));
    Ok((root, intervals, weight, body.to_string()))
}

/// Lays chords out over the work: each takes its share of the time.
fn lay_out(chords: Vec<(f64, Vec<f64>, f64, String)>) -> Vec<Chord> {
    let total: f64 = chords.iter().map(|c| c.2).sum();
    let mut start = 0.0;
    let n = chords.len();
    chords
        .into_iter()
        .enumerate()
        .map(|(k, (root, intervals, w, name))| {
            let end = if k + 1 == n { 1.0 } else { start + w / total };
            let c = Chord { start, end, root, intervals, name };
            start = end;
            c
        })
        .collect()
}

/// A written progression: chord names or roman numerals, separated by spaces
/// (or `|` or `-`); the whole list spans the work, repeated `times` times.
pub fn written(text: &str, steps: &[f64], key: f64, times: usize) -> Result<Vec<Chord>, String> {
    let key_pc = semitones(key).round().rem_euclid(12.0);
    let tokens: Vec<&str> = text.split(|c: char| c.is_whitespace() || c == '|' || c == ',').filter(|t| !t.is_empty() && *t != "-").collect();
    if tokens.is_empty() {
        return Err("a progression needs at least one chord".into());
    }
    let mut all = Vec::new();
    for _ in 0..times.max(1) {
        for t in &tokens {
            all.push(read_chord(t, steps, key_pc)?);
        }
    }
    Ok(lay_out(all))
}

/// A drawn progression of `n` chords: tonic, subdominant and dominant
/// functions in a weighted walk, each phrase of four ending on a cadence, the
/// whole ending home. `tension` (0–1) asks for more dominants, sevenths and
/// a major dominant in minor keys.
pub fn functional(rng: &mut impl Rng, steps: &[f64], n: usize, tension: f64) -> Vec<Chord> {
    #[derive(Clone, Copy, PartialEq)]
    enum F {
        T,
        S,
        D,
    }
    let seven = seven(steps);
    let minor = (seven[2] - 3.0).abs() < 1e-9;
    let pick = |rng: &mut dyn rand::RngCore, w: &[(usize, f64)]| -> usize {
        let total: f64 = w.iter().map(|x| x.1).sum();
        let mut u = rng.random::<f64>() * total;
        for (d, x) in w {
            if u < *x {
                return *d;
            }
            u -= x;
        }
        w[w.len() - 1].0
    };
    let next_f = |rng: &mut dyn rand::RngCore, f: F| -> F {
        let w: [(F, f64); 3] = match f {
            F::T => [(F::T, 0.15), (F::S, 0.5), (F::D, 0.35 + 0.2 * tension)],
            F::S => [(F::S, 0.15), (F::D, 0.6 + 0.2 * tension), (F::T, 0.25)],
            F::D => [(F::T, 0.8), (F::D, 0.1 + 0.1 * tension), (F::S, 0.1)],
        };
        let total: f64 = w.iter().map(|x| x.1).sum();
        let mut u = rng.random::<f64>() * total;
        for (g, x) in w {
            if u < x {
                return g;
            }
            u -= x;
        }
        F::T
    };
    let degree_of = |rng: &mut dyn rand::RngCore, f: F| -> usize {
        match f {
            F::T => pick(rng, &[(0, 0.5), (5, 0.35), (2, 0.15)]),
            F::S => pick(rng, &[(3, 0.55), (1, 0.45)]),
            F::D => pick(rng, &[(4, 0.8), (6, 0.2)]),
        }
    };
    let n = n.max(1);
    let mut degrees: Vec<usize> = Vec::with_capacity(n);
    let mut f = F::T;
    for k in 0..n {
        let phrase_end = k % 4 == 3;
        let d = if k == 0 || (k + 1 == n && n > 1) {
            f = F::T;
            0
        } else if k + 2 == n {
            f = F::D; // the authentic cadence: V, then home
            4
        } else if phrase_end && rng.random::<f64>() < 0.6 {
            f = F::D; // a half cadence ends the phrase
            4
        } else {
            f = next_f(rng, f);
            let mut d = degree_of(rng, f);
            if degrees.last() == Some(&d) {
                d = degree_of(rng, f);
            }
            d
        };
        degrees.push(d);
    }
    let chords: Vec<(f64, Vec<f64>, f64, String)> = degrees
        .iter()
        .map(|&d| {
            let dominant = d == 4;
            let seventh = rng.random::<f64>() < tension * if dominant { 0.8 } else { 0.35 };
            let mut iv = diatonic(&seven, d, seventh);
            // in a minor key the dominant is often made major (harmonic minor)
            if dominant && minor && rng.random::<f64>() < 0.5 + 0.5 * tension {
                iv[1] = 4.0;
            }
            let name = roman_name(d, &iv);
            (seven[d], iv, 1.0, name)
        })
        .collect();
    lay_out(chords)
}

// ───────────────────────────── arpeggios ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArpMode {
    Up,
    Down,
    UpDown,
    Random,
    Converge,
}

pub const ARP_MODES: &[&str] = &["up", "down", "updown", "random", "converge"];

impl ArpMode {
    pub fn parse(s: &str) -> Option<ArpMode> {
        Some(match s {
            "up" => ArpMode::Up,
            "down" => ArpMode::Down,
            "updown" => ArpMode::UpDown,
            "random" => ArpMode::Random,
            "converge" => ArpMode::Converge,
            _ => return None,
        })
    }

    /// Which step of a ladder of `len` notes the `r`-th note of the arpeggio plays.
    pub fn index(self, r: usize, len: usize, seed: u64) -> usize {
        let len = len.max(1);
        match self {
            ArpMode::Up => r % len,
            ArpMode::Down => len - 1 - r % len,
            ArpMode::UpDown => {
                if len == 1 {
                    return 0;
                }
                let p = r % (2 * len - 2);
                if p < len { p } else { 2 * len - 2 - p }
            }
            ArpMode::Random => (crate::eval::hash(&[seed, 0xA2B, r as u64]) % len as u64) as usize,
            ArpMode::Converge => {
                let p = r % len;
                if p % 2 == 0 { p / 2 } else { len - 1 - p / 2 }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arp {
    pub mode: ArpMode,
    pub octaves: usize,
    /// How long each note is held, as a fraction of the time to the next.
    pub gate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!((note_hz("A4").unwrap() - 440.0).abs() < 1e-9);
        assert!((note_hz("C4").unwrap() - 261.6256).abs() < 1e-3);
        assert!((note_hz("Bb2").unwrap() - 116.5409).abs() < 1e-3);
        assert!(note_hz("H2").is_none());
    }

    #[test]
    fn numerals_and_names() {
        let minor = [0., 2., 3., 5., 7., 8., 10.];
        let c = written("i iv V7 i", &minor, 220.0, 1).unwrap();
        assert_eq!(c[2].root, 7.0);
        assert_eq!(c[2].intervals, vec![0., 4., 7., 10.]);
        assert_eq!(c[1].intervals, vec![0., 3., 7.]);
        let n = written("Am F C G", &minor, 220.0, 1).unwrap();
        assert_eq!(n[1].root, 8.0); // F above A
        assert_eq!(n[3].start, 0.75);
        assert!(written("i bVII bVI V", &minor, 220.0, 1).is_ok());
    }

    #[test]
    fn functional_ends_home() {
        let mut rng = <rand_chacha::ChaCha8Rng as rand::SeedableRng>::seed_from_u64(3);
        let major = [0., 2., 4., 5., 7., 9., 11.];
        for _ in 0..20 {
            let c = functional(&mut rng, &major, 8, 0.4);
            assert_eq!(c.len(), 8);
            assert_eq!(c[0].root, 0.0);
            assert_eq!(c[7].root, 0.0);
            assert_eq!(c[6].root, 7.0);
        }
    }

    #[test]
    fn voices_move_little() {
        let major = [0., 2., 4., 5., 7., 9., 11.];
        let h = Harmony { key: 220.0, chords: written("I IV V I", &major, 220.0, 1).unwrap(), tempo: None };
        let v = h.voicing(3, 220.0);
        for voice in &v {
            for w in voice.windows(2) {
                assert!((w[1].1 - w[0].1).abs() <= 7.0, "{voice:?}");
            }
        }
    }
}
