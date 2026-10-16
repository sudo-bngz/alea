use alea::{error, eval, library, media, render};

mod export;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
alea — a language where every run produces a different artwork

usage: alea <file.alea> [options]

options:
  -s, --seed N       replay a specific edition (decimal or 0x…)
  -n, --editions N   produce N editions in a row (default: 1)
  -o, --out DIR      output directory (default: .)
      --check        parse and resolve the work without writing anything
      --web FILE     write a web page that draws a new edition on every visit
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
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("`{a}` expects a value\n\n{USAGE}"));
        match a.as_str() {
            "-s" | "--seed" => seed = Some(parse_seed(&value()?)?),
            "-n" | "--editions" => editions = value()?.parse().map_err(|_| "invalid number of editions\n")?,
            "-o" | "--out" => out = PathBuf::from(value()?),
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
    let module = library::load(&file).map_err(|e| e.display("", &file))?;
    let show = |e: error::Error| e.display(&module.src, &file);
    let work = &module.work;

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

    std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}\n", out.display()))?;
    let slug = work.name.to_lowercase();
    for k in 0..editions {
        let s = seed.map(|b| b.wrapping_add(k)).unwrap_or_else(rand::random);
        let ed = eval::resolve(&module, s).map_err(show)?;
        let files = render::render_all(&ed, &out, &slug).map_err(show)?;
        let attempts =
            if ed.attempts > 1 { format!(" — {} attempts to satisfy the constraints", ed.attempts) } else { String::new() };
        println!("`{}` — edition 0x{:016x}{attempts}", ed.name, ed.seed);
        for (kind, path) in files {
            println!("  {kind:<10} {}", path.display());
        }
    }
    Ok(())
}
