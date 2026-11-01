//! The FM-1: alea synths as patches for the ALEA engine of the M-VAVE FM-1
//! (a Felucca firmware engine, `fm1/`). A `synth { … }`, compiled, becomes a
//! small bytecode that the engine's fixed-point interpreter (`fm1/alea_vm.h`)
//! plays, one voice per note; every `~` in the patch has been drawn, so each
//! edition of a work is one patch, and a bank holds up to sixteen of them.
//!
//! The format is in `fm1/FORMAT.md`. Numbers are Q16.16 (1.0 = 65536),
//! little-endian; slots and states are indexes into the program.

use crate::synth::{NoteValue, Op, Op2, Pass, State, Wave};

/// The engine's sample rate (Felucca's).
pub const FS: u32 = 44100;
/// Patches in a bank, at most.
pub const BANK_SLOTS: usize = 16;
/// What one voice may use for its delay and chorus lines, in bytes
/// (eight voices share the FM-1's pool with everything else).
pub const POOL_PER_VOICE: usize = 24 * 1024;

fn q16(x: f64) -> i32 {
    (x * 65536.0).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// A patch: its program as bytes, and warnings about what the FM-1 will
/// struggle with.
pub(crate) fn encode(ops: &[Op], states: &[State]) -> (Vec<u8>, Vec<String>) {
    let mut b = Vec::new();
    let mut warnings = Vec::new();
    let u16 = |b: &mut Vec<u8>, v: usize| b.extend((v as u16).to_le_bytes());
    let i32 = |b: &mut Vec<u8>, v: i32| b.extend(v.to_le_bytes());
    let u32 = |b: &mut Vec<u8>, v: usize| b.extend((v as u32).to_le_bytes());
    // the lines, in samples at the engine's rate
    let pool: usize = states
        .iter()
        .map(|s| match s {
            State::Line { buf, .. } => buf.len() * FS as usize / crate::sound::SR as usize * 2,
            State::Chorus { .. } => (FS as usize * 5 / 100) * 2,
            _ => 0,
        })
        .sum();
    if pool > POOL_PER_VOICE {
        warnings.push(format!(
            "the patch's delay and chorus lines need {} KB per voice; the FM-1 has {} KB for each of its eight voices: \
             the engine will shorten the delay (use Felucca's DELAY send for long echoes)",
            pool / 1024,
            POOL_PER_VOICE / 1024
        ));
    }
    b.extend(b"ALP1");
    u16(&mut b, FS as usize);
    u16(&mut b, ops.len());
    u16(&mut b, states.len());
    u32(&mut b, pool);
    for op in ops {
        match op {
            Op::Const(c) => {
                b.push(0);
                i32(&mut b, q16(*c));
            }
            Op::Note(v) => {
                b.push(1);
                b.push(match v {
                    NoteValue::Freq => 0,
                    NoteValue::Velocity => 1,
                    NoteValue::Gate => 2,
                    NoteValue::Time => 3,
                    NoteValue::Level => 4,
                });
            }
            Op::Osc { wave, freq, width, state } => {
                b.push(2);
                b.push(match wave {
                    Wave::Sine => 0,
                    Wave::Saw => 1,
                    Wave::Square => 2,
                    Wave::Triangle => 3,
                    Wave::Pulse => 4,
                });
                u16(&mut b, *freq);
                u16(&mut b, *width);
                u16(&mut b, *state);
            }
            Op::Noise => b.push(3),
            Op::Adsr { attack, decay, sustain, release, state } => {
                b.push(4);
                for v in [attack, decay, sustain, release] {
                    i32(&mut b, q16(*v));
                }
                u16(&mut b, *state);
            }
            Op::Perc { attack, release } => {
                b.push(5);
                i32(&mut b, q16(*attack));
                i32(&mut b, q16(*release));
            }
            Op::Filter { pass, input, cutoff, resonance, state } => {
                b.push(6);
                b.push(match pass {
                    Pass::Low => 0,
                    Pass::High => 1,
                    Pass::Band => 2,
                });
                u16(&mut b, *input);
                u16(&mut b, *cutoff);
                u16(&mut b, *resonance);
                u16(&mut b, *state);
            }
            Op::Delay { input, len, feedback, mix, state } => {
                b.push(7);
                u16(&mut b, *input);
                u32(&mut b, (*len * FS as usize / crate::sound::SR as usize).max(1));
                i32(&mut b, q16(*feedback));
                i32(&mut b, q16(*mix));
                u16(&mut b, *state);
            }
            Op::Drive { input, amount } => {
                b.push(8);
                u16(&mut b, *input);
                u16(&mut b, *amount);
            }
            Op::Chorus { input, depth, rate, mix, state } => {
                b.push(9);
                u16(&mut b, *input);
                i32(&mut b, q16(*depth));
                i32(&mut b, q16(*rate));
                i32(&mut b, q16(*mix));
                u16(&mut b, *state);
            }
            Op::Bin(op, x, y) => {
                b.push(match op {
                    Op2::Add => 10,
                    Op2::Sub => 11,
                    Op2::Mul => 12,
                    Op2::Div => 13,
                });
                u16(&mut b, *x);
                u16(&mut b, *y);
            }
            Op::Neg(x) => {
                b.push(14);
                u16(&mut b, *x);
            }
        }
    }
    for s in states {
        match s {
            State::Phase(_) => {
                b.push(0);
                u32(&mut b, 0);
            }
            State::Env { .. } => {
                b.push(1);
                u32(&mut b, 0);
            }
            State::Svf { .. } => {
                b.push(2);
                u32(&mut b, 0);
            }
            State::Line { buf, .. } => {
                b.push(3);
                u32(&mut b, (buf.len() * FS as usize / crate::sound::SR as usize).max(1));
            }
            State::Chorus { .. } => {
                b.push(4);
                u32(&mut b, FS as usize * 5 / 100);
            }
        }
    }
    (b, warnings)
}

/// A bank: up to sixteen named patches, as the engine stores them.
pub fn bank(patches: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(b"ALB1");
    b.push(patches.len().min(BANK_SLOTS) as u8);
    for (name, bytes) in patches.iter().take(BANK_SLOTS) {
        let mut n = [0u8; 16];
        for (k, c) in name.bytes().filter(|c| c.is_ascii_graphic() || *c == b' ').take(15).enumerate() {
            n[k] = c.to_ascii_uppercase();
        }
        b.extend(n);
        b.extend((bytes.len() as u32).to_le_bytes());
        b.extend(bytes);
    }
    b
}

/// The bank as a SysEx file for the FM-1's ALEA engine: F0 7D 'A' 'L' 'E' 'A' 01,
/// the bytes packed seven to eight (each group of seven bytes preceded by
/// their high bits), F7. The firmware unpacks and stores it in flash.
pub fn sysex(bank: &[u8]) -> Vec<u8> {
    let mut out = vec![0xF0, 0x7D, b'A', b'L', b'E', b'A', 0x01];
    for chunk in bank.chunks(7) {
        let mut high = 0u8;
        for (k, c) in chunk.iter().enumerate() {
            if c & 0x80 != 0 {
                high |= 1 << k;
            }
        }
        out.push(high);
        out.extend(chunk.iter().map(|c| c & 0x7F));
    }
    out.push(0xF7);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `body` (the inside of a `synth { … }`) both ways: alea's f64
    /// voice, and the C interpreter through `fm1/test/vm_main.c`, as one note.
    /// None when there is no C compiler to build the harness with.
    fn both_ways(body: &str, freq: f64, vel: f64, hold: f64, total: f64) -> Option<(Vec<f64>, Vec<f64>)> {
        if cfg!(windows) {
            return None; // the harness writes binary to stdout, and the FM-1's toolchain is Linux anyway
        }
        let dir = std::env::temp_dir().join(format!("alea-fm1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let exe = dir.join("vm_main");
        if !exe.exists() {
            let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
            let ok = std::process::Command::new(&cc)
                .args(["-std=c99", "-O2", "-o"])
                .arg(&exe)
                .arg(root.join("fm1/test/vm_main.c"))
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                eprintln!("no C compiler: the FM-1 interpreter isn't checked");
                return None;
            }
        }
        let src = format!("work W {{\n  output synth {{\n{body}\n  }}\n}}\n");
        let path = dir.join(format!("w{:x}.alea", crate::eval::hash(&[body.len() as u64, body.bytes().map(|b| b as u64).sum()])));
        std::fs::write(&path, src).unwrap();
        let m = crate::library::load(path.to_str().unwrap()).unwrap();
        let synth = crate::eval::resolve_synth(&m, 1).unwrap();
        let ours: Vec<f64> = synth.note(freq, vel, hold, 1).into_iter().chain(std::iter::repeat(0.0)).take((total * FS as f64) as usize).collect();
        let (bytes, _) = synth.fm1_bytes();
        let alp = path.with_extension("alp");
        std::fs::write(&alp, bytes).unwrap();
        let out = std::process::Command::new(&exe)
            .arg(&alp)
            .args([freq.to_string(), vel.to_string(), hold.to_string(), total.to_string()])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let theirs: Vec<f64> = out.stdout.chunks(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64 / 65536.0).collect();
        Some((ours, theirs))
    }

    fn rms(v: &[f64]) -> f64 {
        (v.iter().map(|x| x * x).sum::<f64>() / v.len().max(1) as f64).sqrt()
    }

    /// The loudness along the note, in 20 ms windows, both ways, as how far apart they are (0 = the same).
    fn envelope_gap(a: &[f64], b: &[f64]) -> f64 {
        let w = FS as usize / 50;
        let (ea, eb): (Vec<f64>, Vec<f64>) = (a.chunks(w).map(rms).collect(), b.chunks(w).map(rms).collect());
        let top = ea.iter().cloned().fold(0.0, f64::max).max(1e-9);
        ea.iter().zip(&eb).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max) / top
    }

    fn pitch(v: &[f64]) -> f64 {
        let ups: Vec<usize> = (1..v.len()).filter(|&k| v[k - 1] < 0.0 && v[k] >= 0.0).collect();
        (ups.len() - 1) as f64 / ((ups[ups.len() - 1] - ups[0]) as f64 / FS as f64)
    }

    /// `ALEA_FM1_DEBUG='<body>' cargo test -- fm1::tests::debug --nocapture`: the loudness both ways, along a note.
    #[test]
    fn debug() {
        let Ok(body) = std::env::var("ALEA_FM1_DEBUG") else { return };
        let (a, b) = both_ways(&body, 440.0, 0.8, 0.1, 2.0).unwrap();
        let w = FS as usize / 20;
        for (k, (x, y)) in a.chunks(w).zip(b.chunks(w)).enumerate() {
            println!("t={:.2}  alea {:.4}  c {:.4}", k as f64 * 0.05, rms(x), rms(y));
        }
    }

    /// The engine builds against Felucca's engine interface (a copy of its header, MIT, in fm1/test/)
    /// with a mock host, loads a bank and plays a note that ends.
    #[test]
    fn the_engine_builds_for_felucca() {
        if cfg!(windows) {
            return;
        }
        let dir = std::env::temp_dir().join(format!("alea-fm1-host-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let exe = dir.join("host_main");
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        let built = std::process::Command::new(&cc)
            .args(["-std=c99", "-O2", "-o"])
            .arg(&exe)
            .arg(root.join("fm1/test/host_main.c"))
            .arg("-lm")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !built {
            eprintln!("no C compiler: the FM-1 engine isn't built");
            return;
        }
        let src = "work Bass {\n  output synth {\n    lowpass(saw() + square(note.freq * 0.5) * 0.5, cutoff: 300 + 2000 * perc(release: 0.3), resonance: 0.4) \
                   * adsr(attack: 0.005, decay: 0.2, sustain: 0.5, release: 0.15)\n  }\n}\n";
        let path = dir.join("bass.alea");
        std::fs::write(&path, src).unwrap();
        let m = crate::library::load(path.to_str().unwrap()).unwrap();
        let mut patches = Vec::new();
        for seed in 1..=3 {
            let synth = crate::eval::resolve_synth(&m, seed).unwrap();
            patches.push((format!("bass {seed}"), synth.fm1_bytes().0));
        }
        let alb = dir.join("bass.alb");
        std::fs::write(&alb, bank(&patches)).unwrap();
        let out = std::process::Command::new(&exe).arg(&alb).args(["0.4", "2.0"]).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}{}", String::from_utf8_lossy(&out.stderr));
        assert!(text.contains("bank: 3 patches, first `BASS 1` ok=1"), "{text}");
        assert!(text.contains("done at 0."), "the voice ends after its release: {text}");
        let rms: f64 = text.split("rms ").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
        assert!(rms > 3000.0 && rms < 32767.0, "a note at a sensible level: {rms}");
    }

    #[test]
    fn the_interpreter_plays_what_alea_plays() {
        // a sine: the same samples, near enough, over the whole note
        let Some((a, b)) = both_ways("    sine() * adsr(attack: 0.01, decay: 0.1, sustain: 0.6, release: 0.2)", 220.0, 1.0, 0.5, 0.9) else { return };
        assert!((pitch(&a[2000..20000]) - 220.0).abs() < 0.5 && (pitch(&b[2000..20000]) - 220.0).abs() < 0.5);
        // (after the 2 ms declick alea's rendered note has, which the engine's host does its own way)
        let (at, err) = a.iter().zip(&b).enumerate().skip(100).map(|(k, (x, y))| (k, (x - y).abs())).fold((0, 0.0), |m, e| if e.1 > m.1 { e } else { m });
        assert!(err < 0.02, "sine: samples differ by up to {err} at sample {at} ({} vs {})", a[at], b[at]);
        assert!(rms(&b[(0.75 * FS as f64) as usize..]) < 1e-3, "silent after the release");

        // a Juno-style bass: oscillators, a filter that moves, drive
        let body = "    let env = adsr(attack: 0.004, decay: 0.2, sustain: 0.5, release: 0.15)\n    \
                    drive(lowpass(saw() + pulse(note.freq, width: 0.3) * 0.5 + square(note.freq * 0.5) * 0.6, \
                    cutoff: 200 + 2500 * perc(release: 0.25) * note.velocity, resonance: 0.4) * env, amount: 0.25)";
        let (a, b) = both_ways(body, 110.0, 0.9, 0.5, 1.0).unwrap();
        let gap = envelope_gap(&a, &b);
        assert!(gap < 0.12, "bass: the loudness along the note differs by {gap}");
        assert!((rms(&a) - rms(&b)).abs() / rms(&a) < 0.1, "bass: rms {} vs {}", rms(&a), rms(&b));

        // a struck bell through a delay and a chorus: the echoes and the tail
        let body = "    delay(chorus(sine(note.freq + sine(note.freq * 2.76) * perc(release: 0.5) * note.freq) * perc(attack: 0.002, release: 1.2), \
                    depth: 0.6), time: 0.25, feedback: 0.4, mix: 0.4)";
        let (a, b) = both_ways(body, 440.0, 0.8, 0.1, 2.5).unwrap();
        let gap = envelope_gap(&a, &b);
        assert!(gap < 0.15, "bell: the loudness along the note differs by {gap}");
        let late = |v: &[f64]| rms(&v[(1.6 * FS as f64) as usize..(2.0 * FS as f64) as usize]);
        assert!(late(&b) > 0.2 * late(&a) && late(&b) < 5.0 * late(&a), "the echoes go on both ways: {} vs {}", late(&a), late(&b));

        // noise through a filter: the same loudness, the same colour
        let (a, b) = both_ways("    highpass(noise(), cutoff: 6000, resonance: 0.3) * perc(release: 0.1)", 440.0, 1.0, 0.1, 0.4).unwrap();
        assert!((rms(&a) - rms(&b)).abs() / rms(&a) < 0.25, "hat: rms {} vs {}", rms(&a), rms(&b));
    }

    #[test]
    fn sysex_packs_seven_bits_and_unpacks_back() {
        let bank: Vec<u8> = (0..300u32).map(|k| (k * 37 % 256) as u8).collect();
        let sx = sysex(&bank);
        assert_eq!(sx[0], 0xF0);
        assert_eq!(*sx.last().unwrap(), 0xF7);
        assert!(sx[1..sx.len() - 1].iter().all(|b| *b < 0x80), "only data bytes inside");
        // unpack, as the firmware does
        let body = &sx[7..sx.len() - 1];
        let mut back = Vec::new();
        for g in body.chunks(8) {
            for (k, c) in g[1..].iter().enumerate() {
                back.push(c | if g[0] & (1 << k) != 0 { 0x80 } else { 0 });
            }
        }
        assert_eq!(back, bank);
    }

    #[test]
    fn a_bank_names_its_patches() {
        let b = bank(&[("bass line".into(), vec![1, 2, 3]), ("pad".into(), vec![4])]);
        assert_eq!(&b[0..4], b"ALB1");
        assert_eq!(b[4], 2);
        assert_eq!(&b[5..14], b"BASS LINE");
        assert_eq!(u32::from_le_bytes([b[21], b[22], b[23], b[24]]), 3);
    }
}
