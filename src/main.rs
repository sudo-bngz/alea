use alea::{error, eval, input, library, live, media, render};

mod export;

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
alea — a language where every run produces a different artwork

usage: alea <file.alea> [options]
       cat photo.png | alea <file.alea> > out.png

options:
  -s, --seed N         replay a specific edition (decimal or 0x…)
  -n, --editions N     produce N editions in a row (default: 1)
  -o, --out DIR        output directory (default: .); `-o -` writes the first render to the standard output
      --in FILE        the work's input (an image, a WAV sound or a text), instead of the pipe;
                       `--in -` waits for the pipe however long it takes (otherwise: 3 seconds)
      --in NAME=FILE   a named input, read with `input(\"NAME\")`
      --seed-from-input  the edition is decided by the input: the same input, the same edition
      --midi           play the work's music live, as MIDI, on a new port called \"alea\"
      --midi-port P    play to an existing port whose name contains P (`print`: as text)
      --midi-ports     list the MIDI ports
      --loop           with --midi: a new edition each time one ends, until Ctrl-C
      --sync           with --midi: follow a DAW's MIDI clock (start, stop, tempo)
      --sync-port P    the port the clock comes from (otherwise a new port, \"alea clock\")
      --check          parse and resolve the work without writing anything
      --web FILE       write a web page that draws a new edition on every visit

Piped in, an input reaches the work's `input()`. Piped out (when the standard
output is not a terminal and no -o is given), the first render goes to it, so
works chain: cat a.png | alea blur.alea | alea frame.alea > b.png
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprint!("{msg}");
            ExitCode::FAILURE
        }
    }
}

/// Whether the standard input has something to read (or has ended) within `ms`.
#[cfg(unix)]
fn stdin_ready(ms: i32) -> bool {
    let mut fd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
    // SAFETY: one valid pollfd, for the duration of the call.
    let n = unsafe { libc::poll(&mut fd, 1, ms) };
    n > 0
}

#[cfg(not(unix))]
fn stdin_ready(_ms: i32) -> bool {
    true
}

fn parse_seed(s: &str) -> Result<u64, String> {
    let r = match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    };
    r.map_err(|_| format!("invalid seed `{s}`\n"))
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut file, mut seed, mut editions, mut out, mut check) = (None, None, 1u64, PathBuf::from("."), false);
    let mut web: Option<PathBuf> = None;
    let (mut out_given, mut from_input) = (false, false);
    let mut inputs: Vec<(String, PathBuf)> = Vec::new();
    let (mut midi, mut midi_port, mut looping, mut sync, mut sync_port): (bool, Option<String>, bool, bool, Option<String>) =
        (false, None, false, false, None);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("`{a}` expects a value\n\n{USAGE}"));
        match a.as_str() {
            "-s" | "--seed" => seed = Some(parse_seed(&value()?)?),
            "-n" | "--editions" => editions = value()?.parse().map_err(|_| "invalid number of editions\n")?,
            "-o" | "--out" => {
                out = PathBuf::from(value()?);
                out_given = true;
            }
            "--in" => {
                let v = value()?;
                let named = v.split_once('=').filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_'));
                match named {
                    Some((n, f)) => inputs.push((n.to_string(), PathBuf::from(f))),
                    None => inputs.push((String::new(), PathBuf::from(v))),
                }
            }
            "--seed-from-input" => from_input = true,
            "--midi" => midi = true,
            "--midi-port" => {
                midi = true;
                midi_port = Some(value()?);
            }
            "--loop" => looping = true,
            "--sync" => sync = true,
            "--sync-port" => {
                sync = true;
                sync_port = Some(value()?);
            }
            "--midi-ports" => return list_ports(),
            "--check" => check = true,
            "--web" => web = Some(PathBuf::from(value()?)),
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            _ if file.is_none() && !a.starts_with('-') => file = Some(a),
            _ => return Err(format!("unexpected argument `{a}`\n\n{USAGE}")),
        }
    }
    let file = file.ok_or(USAGE.to_string())?;

    // Inputs: files given with --in, otherwise the pipe (read only if the work asks).
    for (name, path) in &inputs {
        if name.is_empty() && path.as_os_str() == "-" {
            continue;
        }
        let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}\n", path.display()))?;
        input::put(name, bytes).map_err(|e| format!("{}: {e}\n", path.display()))?;
    }
    let wait = inputs.iter().any(|(n, p)| n.is_empty() && p.as_os_str() == "-");
    inputs.retain(|(n, p)| !(n.is_empty() && p.as_os_str() == "-"));
    if !inputs.iter().any(|(n, _)| n.is_empty()) && !std::io::stdin().is_terminal() {
        input::set_stdin_reader(Box::new(move || {
            // A pipe that stays silent (a script, a scheduler) is not an input:
            // wait a few seconds for its first byte, or for ever with `--in -`.
            if !wait && !stdin_ready(3000) {
                return None;
            }
            let mut bytes = Vec::new();
            std::io::stdin().lock().read_to_end(&mut bytes).ok()?;
            Some(bytes)
        }));
    }
    if from_input && seed.is_none() {
        let i = input::get("").map_err(|e| format!("{e}\n"))?.ok_or("--seed-from-input: there is no input (pipe one in, or use --in)\n")?;
        seed = Some(i.hash);
    }
    // Piped out: the first render goes to the standard output, messages to the error output.
    let to_stdout = out.as_os_str() == "-" || (!out_given && !std::io::stdout().is_terminal());
    if to_stdout && editions > 1 {
        return Err("only one edition can go to the standard output: drop -n, or write files with -o DIR\n".into());
    }
    let module = library::load(&file).map_err(|e| e.display("", &file))?;
    let show = |e: error::Error| e.display(&module.src, &file);
    let work = &module.work;

    if midi {
        return play_live(&module, seed, looping, midi_port.as_deref(), sync.then_some(sync_port.as_deref()));
    }
    if looping || sync {
        return Err("--loop and --sync go with --midi\n".into());
    }

    if let Some(page) = web {
        // Draw once, so every render is checked and every picture it reads is known.
        let ed = eval::resolve(&module, seed.unwrap_or(0)).map_err(show)?;
        media::validate(&ed).map_err(show)?;
        return export::export(&module, &page);
    }

    if check {
        let ed = eval::resolve(&module, seed.unwrap_or(0)).map_err(show)?;
        println!("`{}` is well-formed ({} render(s)).", work.name, ed.renders.len());
        return Ok(());
    }

    let dir = if to_stdout {
        std::env::temp_dir().join(format!("alea-{}", std::process::id()))
    } else {
        out.clone()
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}\n", dir.display()))?;
    let slug = work.name.to_lowercase();
    for k in 0..editions {
        let s = seed.map(|b| b.wrapping_add(k)).unwrap_or_else(rand::random);
        let ed = eval::resolve(&module, s).map_err(show)?;
        let files = render::render_all(&ed, &dir, &slug).map_err(show)?;
        let attempts =
            if ed.attempts > 1 { format!(" — {} attempts to satisfy the constraints", ed.attempts) } else { String::new() };
        let ins: String = ed
            .inputs
            .iter()
            .map(|i| {
                let name = if i.name.is_empty() { String::new() } else { format!("{} ", i.name) };
                format!(" · input {name}{:08x}", i.hash >> 32)
            })
            .collect();
        let line = format!("`{}` — edition 0x{:016x}{ins}{attempts}", ed.name, ed.seed);
        if to_stdout {
            eprintln!("{line}");
            let (kind, path) = files.first().ok_or("this work has no render to write\n")?;
            let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}\n", path.display()))?;
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&bytes).and_then(|_| stdout.flush()).map_err(|e| format!("writing the {kind}: {e}\n"))?;
            if files.len() > 1 {
                eprintln!("  (the first of {} renders went to the standard output; use -o DIR for all of them)", files.len());
            }
            let _ = std::fs::remove_dir_all(&dir);
        } else {
            println!("{line}");
            for (kind, path) in files {
                println!("  {kind:<10} {}", path.display());
            }
        }
    }
    Ok(())
}

#[cfg(feature = "midi")]
fn list_ports() -> Result<(), String> {
    print!("{}", live::list().map_err(|e| format!("{e}\n"))?);
    Ok(())
}

#[cfg(not(feature = "midi"))]
fn list_ports() -> Result<(), String> {
    Err(NO_MIDI.into())
}

#[cfg_attr(feature = "midi", allow(dead_code))]
const NO_MIDI: &str = "this alea was built without live MIDI: build it with `cargo build --release --features midi` \
(on Linux, ALSA's headers first: `sudo apt install libasound2-dev`). `--midi-port print` works without it.\n";

/// Plays editions live: one, or one after another with `--loop`.
fn play_live(
    module: &std::sync::Arc<library::Module>,
    seed: Option<u64>,
    looping: bool,
    port: Option<&str>,
    sync: Option<Option<&str>>,
) -> Result<(), String> {
    let show = |e: error::Error| e.display(&module.src, &module.file);
    let (mut sink, shown): (Box<dyn live::Sink>, String) = match port {
        Some("print") => (Box::new(live::Printer::new()), "the standard output, as text".into()),
        #[cfg(feature = "midi")]
        other => live::output(other).map_err(|e| {
            format!("opening MIDI: {e}\n(on Linux, the ALSA sequencer must be there: `ls /dev/snd/seq`, else `sudo modprobe snd-seq`)\n")
        })?,
        #[cfg(not(feature = "midi"))]
        _ => return Err(NO_MIDI.into()),
    };
    #[allow(unused_variables)]
    let state = std::sync::Arc::new(live::SyncState::default());
    let timing = match sync {
        None => live::Timing::Free,
        #[cfg(feature = "midi")]
        Some(p) => {
            let (conn, from) = live::clock(p, state.clone()).map_err(|e| format!("listening to the clock: {e}\n"))?;
            eprintln!("following the MIDI clock on {from}");
            std::mem::forget(conn); // kept open for as long as alea plays
            live::Timing::Sync(state.clone())
        }
        #[cfg(not(feature = "midi"))]
        Some(_) => return Err(NO_MIDI.into()),
    };
    live::catch_ctrl_c();
    eprintln!("playing to {shown} (Ctrl-C to stop)");

    // Each edition's score is prepared while the one before it plays.
    let prepare = |s: u64| -> Result<(eval::Edition, alea::midi::Score), String> {
        let ed = eval::resolve(module, s).map_err(show)?;
        let score = media::live_score(&ed)
            .ok_or_else(|| "this work has no music to play: give it `render midi(…)`, or sound layers (`render sound(drone(…), notes(…))`)\n".to_string())?
            .map_err(show)?;
        Ok((ed, score))
    };
    let seed_of = |k: u64| seed.map(|b| b.wrapping_add(k)).unwrap_or_else(rand::random);
    let mut k = 0u64;
    let mut next = prepare(seed_of(0))?;
    let mut base = 0.0;
    if let live::Timing::Sync(c) = &timing {
        if !c.running.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!("waiting for the DAW to start…");
        }
    }
    loop {
        let (ed, score) = next;
        let tempo = match &timing {
            live::Timing::Sync(c) => c.tempo().map(|t| format!(", at the DAW's {t:.0} BPM")).unwrap_or_default(),
            live::Timing::Free => String::new(),
        };
        eprintln!(
            "`{}` — edition 0x{:016x} · {} track(s), {:.1} s{tempo}",
            ed.name,
            ed.seed,
            score.tracks.len(),
            score.duration
        );
        let upcoming = looping.then(|| {
            let (m, s) = (module.clone(), seed_of(k + 1));
            std::thread::spawn(move || {
                let ed = eval::resolve(&m, s).map_err(|e| e.display(&m.src, &m.file))?;
                let score = media::live_score(&ed).ok_or("no music")?.map_err(|e| e.display(&m.src, &m.file))?;
                Ok::<_, String>((ed, score))
            })
        });
        let outcome = live::play(&score, sink.as_mut(), &timing, &mut base)?;
        match outcome {
            live::Outcome::Stopped => {
                eprintln!("stopped");
                return Ok(());
            }
            live::Outcome::Restart => {
                base = 0.0;
                next = (ed, score); // the DAW went back to the start: the same edition again
                continue;
            }
            live::Outcome::Finished => {}
        }
        match upcoming {
            Some(h) => {
                next = h.join().map_err(|_| "preparing the next edition failed\n".to_string())??;
                k += 1;
            }
            None => return Ok(()),
        }
    }
}
