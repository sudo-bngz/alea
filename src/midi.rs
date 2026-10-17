//! MIDI: the music of a work as notes and controllers, for other instruments.
//!
//! The same layers that alea plays (`drone`, `notes`) become MIDI: each layer
//! a track on its own channel, with the notes alea would play, at the same
//! moments, with the same pitches. `cc(field, number)` turns a field into a
//! controller curve, to move a synthesizer's knobs. The result is a score: a
//! list of timed messages, written as a Standard MIDI File (`render midi`) or
//! played live (`alea work.alea --midi`).

use std::sync::Arc;

use crate::sound::{Layer, Scale, Strike, Voice};

/// One MIDI message at a moment, in seconds from the start.
#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub at: f64,
    pub track: usize,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct Score {
    /// One name per track (one track per layer).
    pub tracks: Vec<String>,
    /// In time order (at the same moment: controllers, note-offs, then note-ons).
    pub msgs: Vec<Msg>,
    /// Beats per minute, for the file's grid and for `--sync`.
    pub tempo: f64,
    pub duration: f64,
}

/// General MIDI programs (0-based) for alea's instruments.
fn program_of_voice(v: Voice) -> u8 {
    match v {
        Voice::Sine => 89,    // warm pad
        Voice::Organ => 19,   // church organ
        Voice::Reed => 68,    // oboe
        Voice::Strings => 48, // string ensemble
        Voice::Choir => 52,   // choir aahs
        Voice::Glass => 92,   // bowed glass pad
        Voice::Wind => 91,    // choir pad, airy
    }
}

fn program_of_strike(s: Strike) -> u8 {
    match s {
        Strike::Bell => 14,     // tubular bells
        Strike::Pluck => 24,    // nylon guitar
        Strike::Marimba => 12,  // marimba
        Strike::Kalimba => 108, // kalimba
        Strike::Glass => 9,     // glockenspiel
        Strike::Wood => 115,    // woodblock
        Strike::Drum | Strike::Tick => 0,
    }
}

/// The drum kit (General MIDI, channel 10), from the bottom of the canvas to
/// the top: kick, low tom, snare, clap, mid tom, closed hat, open hat, high
/// tom, ride, crash.
const KIT: &[u8] = &[36, 41, 38, 39, 45, 42, 46, 48, 51, 49];
const TICKS: &[u8] = &[37, 42, 75, 76, 77, 44];

/// A frequency as a MIDI note number (fractional).
fn note_number(freq: f64) -> f64 {
    69.0 + 12.0 * libm::log2(freq / 440.0)
}

/// For a scale of semitones, the whole layer is in tune once the key is: a
/// pitch bend on its channel corrects a key between two notes (bend range ±2).
fn key_bend(scale: &Scale, key: f64) -> Option<u16> {
    match scale {
        Scale::Steps(_) => {
            let n = note_number(key);
            let off = n - n.round();
            if off.abs() < 0.005 {
                None
            } else {
                Some((8192.0 + off / 2.0 * 8192.0).round().clamp(0.0, 16383.0) as u16)
            }
        }
        Scale::Harmonics => None,
    }
}

/// The note a frequency plays on a channel bent by `bend` semitones.
fn midi_note(freq: f64, bend: f64) -> u8 {
    (note_number(freq) - bend).round().clamp(0.0, 127.0) as u8
}

fn bend_semitones(b: Option<u16>) -> f64 {
    b.map_or(0.0, |v| (v as f64 - 8192.0) / 8192.0 * 2.0)
}

/// The score of a list of layers. Textures and sound inputs have no notes:
/// they are left out.
pub fn score(layers: &[Arc<Layer>], duration: f64, tempo: Option<f64>, name: &str) -> Score {
    let mut tracks = Vec::new();
    let mut msgs: Vec<Msg> = Vec::new();
    let mut free = (0u8..16).filter(|c| *c != 9).cycle();
    // the tempo: the one asked for, or the first `notes` layer's, or 120
    let tempo = tempo.filter(|t| *t > 0.0).unwrap_or_else(|| {
        layers
            .iter()
            .find_map(|l| match l.as_ref() {
                Layer::Notes { tempo, .. } if *tempo > 0.0 => Some(*tempo),
                _ => None,
            })
            .unwrap_or(120.0)
    });
    let _ = name;
    for layer in layers {
        let track = tracks.len();
        let mut push = |at: f64, bytes: Vec<u8>| msgs.push(Msg { at, track, bytes });
        match layer.as_ref() {
            Layer::Drone { field, seed, voices, voice, scale, key, harmony, synth, .. } => {
                let ch = free.next().unwrap();
                let instrument = synth.as_ref().map_or(crate::sound::DRONE_INSTRUMENTS[*voice as usize], |s| s.name.as_str());
                tracks.push(format!("drone - {instrument}"));
                push(0.0, vec![0xC0 | ch, program_of_voice(*voice)]);
                let bend = key_bend(scale, *key);
                if let Some(b) = bend {
                    push(0.0, vec![0xE0 | ch, (b & 0x7F) as u8, (b >> 7) as u8]);
                }
                let env = crate::sound::drone_envelopes(field, *seed, *voices, duration);
                let fixed: Vec<u8> =
                    (0..*voices).map(|v| midi_note(scale.freq(*key, crate::sound::drone_degree(scale, v)), bend_semitones(bend))).collect();
                // with a progression, each voice's note moves at the chord changes
                let plan = harmony.as_ref().map(|h| h.voicing(*voices, *key));
                let note_at = |v: usize, pos: f64| match &plan {
                    Some(p) => midi_note(*key * libm::pow(2.0, crate::harmony::pitch_at(&p[v], pos) / 12.0), bend_semitones(bend)),
                    None => fixed[v],
                };
                let mut cur: Vec<u8> = (0..*voices).map(|v| note_at(v, 0.0)).collect();
                // 50 steps per second: a voice sounds while it is loud enough
                // (with some hysteresis), the channel's expression follows the loudest
                let steps = (duration * 50.0).ceil() as usize;
                let mut on = vec![false; *voices];
                let mut last_expr: i32 = -10;
                for k in 0..=steps {
                    let at = (k as f64 / 50.0).min(duration);
                    let pos = at / duration;
                    let mut loudest = 0.0f64;
                    for v in 0..*voices {
                        let e = 0.5 * (sample(&env[v][0], pos) + sample(&env[v][1], pos));
                        loudest = loudest.max(e);
                        let vel = (30.0 + 97.0 * e.sqrt()).round().clamp(1.0, 127.0) as u8;
                        let want = note_at(v, pos);
                        if on[v] && want != cur[v] && k < steps {
                            // the chord changed under a held voice: it moves to its new note
                            push(at, vec![0x80 | ch, cur[v], 0]);
                            cur[v] = want;
                            push(at, vec![0x90 | ch, cur[v], vel]);
                        }
                        cur[v] = want;
                        if !on[v] && e > 0.06 && k < steps {
                            on[v] = true;
                            push(at, vec![0x90 | ch, cur[v], vel]);
                        } else if on[v] && (e < 0.02 || k == steps) {
                            on[v] = false;
                            push(at, vec![0x80 | ch, cur[v], 0]);
                        }
                    }
                    let expr = (loudest.sqrt() * 127.0).round().clamp(0.0, 127.0) as i32;
                    if k % 2 == 0 && (expr - last_expr).abs() >= 2 && k < steps {
                        last_expr = expr;
                        push(at, vec![0xB0 | ch, 11, expr as u8]);
                    }
                }
            }
            Layer::Notes { events, strike, scale, key, tempo: t, feel, seed, synth, .. } => {
                let drums = synth.is_none() && matches!(strike, Strike::Drum | Strike::Tick);
                let ch = if drums { 9 } else { free.next().unwrap() };
                let kind = if feel.arp.is_some() { "arp" } else { "notes" };
                let instrument = synth.as_ref().map_or(crate::sound::NOTE_INSTRUMENTS[*strike as usize], |s| s.name.as_str());
                tracks.push(format!("{kind} - {instrument}"));
                let bend = if drums { None } else { key_bend(scale, *key) };
                if !drums {
                    push(0.0, vec![0xC0 | ch, program_of_strike(*strike)]);
                    if let Some(b) = bend {
                        push(0.0, vec![0xE0 | ch, (b & 0x7F) as u8, (b >> 7) as u8]);
                    }
                }
                for p in crate::sound::placements(events, *strike, scale, *key, *t, duration, feel, *seed) {
                    let e = &events[p.index];
                    if p.when >= duration {
                        continue;
                    }
                    let note = if drums {
                        let set = if *strike == Strike::Drum { KIT } else { TICKS };
                        // the degree climbs the canvas; spread it over the kit
                        let degrees = 3 * scale.len();
                        set[(p.degree * set.len() / degrees.max(1)).min(set.len() - 1)]
                    } else {
                        midi_note(p.freq, bend_semitones(bend))
                    };
                    let vel = (127.0 * (e.intensity * p.vel).clamp(0.0, 1.0)).round().clamp(1.0, 127.0) as u8;
                    let end = (p.when + p.hold.unwrap_or((p.ring * 0.9).max(0.05))).min(duration);
                    push(p.when, vec![0x90 | ch, note, vel]);
                    push(end.max(p.when + 0.01), vec![0x80 | ch, note, 0]);
                }
            }
            Layer::Control { field, seed, number, channel } => {
                let ch = channel.min(&15).to_owned();
                tracks.push(format!("cc {number}"));
                let steps = (duration * 50.0).ceil() as usize;
                let mut last: i32 = -1;
                for k in 0..steps {
                    let at = k as f64 / 50.0;
                    let t = at / duration;
                    // the field over the whole canvas (a curve of time is the same everywhere)
                    let mut acc = 0.0;
                    for j in 0..5 {
                        for i in 0..5 {
                            acc += field.eval([(i as f64 + 0.5) / 5.0, (j as f64 + 0.5) / 5.0, t], *seed);
                        }
                    }
                    let v = ((acc / 25.0).clamp(0.0, 1.0) * 127.0).round() as i32;
                    if v != last {
                        last = v;
                        push(at, vec![0xB0 | ch, *number, v as u8]);
                    }
                }
            }
            Layer::Texture { .. } | Layer::Clip { .. } => {}
        }
    }
    // in time order; at the same moment, controllers and programs first,
    // then note-offs, then note-ons (a repeated note is struck again)
    let rank = |m: &Msg| match m.bytes[0] & 0xF0 {
        0x90 if m.bytes[2] > 0 => 2,
        0x80 | 0x90 => 1,
        _ => 0,
    };
    msgs.sort_by(|a, b| a.at.total_cmp(&b.at).then(rank(a).cmp(&rank(b))));
    Score { tracks, msgs, tempo, duration }
}

fn sample(env: &[f64], pos: f64) -> f64 {
    let p = pos.clamp(0.0, 1.0) * (env.len() - 1) as f64;
    let k = (p as usize).min(env.len() - 2);
    env[k] + (env[k + 1] - env[k]) * (p - k as f64)
}

// ───────────────────────────── the file ─────────────────────────────

/// Ticks per quarter note.
pub const PPQ: u32 = 480;

fn vlq(mut v: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 5];
    let mut n = 0;
    loop {
        buf[n] = (v & 0x7F) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for k in (0..n).rev() {
        out.push(buf[k] | if k > 0 { 0x80 } else { 0 });
    }
}

fn chunk(id: &[u8; 4], body: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
}

fn meta_text(kind: u8, text: &str, out: &mut Vec<u8>) {
    out.extend_from_slice(&[0x00, 0xFF, kind]);
    vlq(text.len() as u32, out);
    out.extend_from_slice(text.as_bytes());
}

/// A Standard MIDI File (format 1): a first track with the tempo and the
/// title, then one track per layer.
pub fn smf(score: &Score, title: &str) -> Vec<u8> {
    let ticks = |s: f64| (s * score.tempo / 60.0 * PPQ as f64).round() as u32;
    let mut out = Vec::new();
    let mut head = Vec::new();
    head.extend_from_slice(&1u16.to_be_bytes());
    head.extend_from_slice(&((score.tracks.len() + 1) as u16).to_be_bytes());
    head.extend_from_slice(&(PPQ as u16).to_be_bytes());
    chunk(b"MThd", &head, &mut out);

    let mut first = Vec::new();
    meta_text(0x03, title, &mut first);
    let us = (60_000_000.0 / score.tempo).round() as u32;
    first.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, (us >> 16) as u8, (us >> 8) as u8, us as u8]);
    first.extend_from_slice(&[0x00, 0xFF, 0x58, 0x04, 4, 2, 24, 8]);
    vlq(ticks(score.duration), &mut first);
    first.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    chunk(b"MTrk", &first, &mut out);

    for (k, name) in score.tracks.iter().enumerate() {
        let mut body = Vec::new();
        meta_text(0x03, name, &mut body);
        let mut now = 0u32;
        for m in score.msgs.iter().filter(|m| m.track == k) {
            let t = ticks(m.at);
            vlq(t - now.min(t), &mut body);
            now = now.max(t);
            body.extend_from_slice(&m.bytes);
        }
        vlq(ticks(score.duration).saturating_sub(now), &mut body);
        body.extend_from_slice(&[0xFF, 0x2F, 0x00]);
        chunk(b"MTrk", &body, &mut out);
    }
    out
}
