//! `imagine(prompt, …)`: a picture from outside, made by a program (an image
//! model, usually) and then kept. The engine is given the prompt and a seed;
//! what it draws is saved in `imagined/`, next to the work, and from then on
//! that file is the picture: the edition renders the same on any machine, with
//! or without the engine, and in the browser.

use std::path::{Path, PathBuf};

/// What an engine is given: the prompt on its standard input, and these in its
/// environment (`ALEA_SEED`, `ALEA_WIDTH`, `ALEA_HEIGHT`, `ALEA_MODEL`). It
/// writes a PNG, a JPEG or a WebP to its standard output.
pub struct Ask<'a> {
    /// The work's folder.
    pub dir: &'a Path,
    pub prompt: &'a str,
    /// The engine, as written in the work (or `$ALEA_IMAGINE`), if any.
    pub engine: Option<&'a str>,
    pub seed: u32,
    pub width: u32,
    pub height: u32,
    pub model: &'a str,
}

pub const FOLDER: &str = "imagined";
const KINDS: [&str; 3] = ["png", "jpg", "webp"];

/// The saved picture's name, without its extension: the first words of the
/// prompt, to recognise it, and a key of everything that was asked.
pub fn stem(a: &Ask) -> String {
    let mut slug = String::new();
    for c in a.prompt.chars() {
        if slug.len() >= 32 {
            break;
        }
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("{}\0{}\0{}\0{}\0{}", a.prompt, a.seed, a.width, a.height, a.model).bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{}{}{:08x}", slug, if slug.is_empty() { "" } else { "-" }, (h ^ (h >> 32)) as u32)
}

fn kind_of(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG") {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// The saved picture, if there is one: its path, and its path as shown.
pub fn saved(a: &Ask) -> Option<(PathBuf, String)> {
    let stem = stem(a);
    KINDS.iter().find_map(|k| {
        let name = format!("{stem}.{k}");
        let full = a.dir.join(FOLDER).join(&name);
        crate::vfs::canonicalize(&full).is_ok().then(|| (full, format!("{FOLDER}/{name}")))
    })
}

/// The picture: the saved one, or the engine's, saved for the next time.
#[cfg(not(target_arch = "wasm32"))]
pub fn picture(a: &Ask) -> Result<(PathBuf, String), String> {
    use std::io::Write;
    if let Some(found) = saved(a) {
        return Ok(found);
    }
    let Some(engine) = a.engine else {
        return Err(format!(
            "this picture hasn't been imagined yet (no `{FOLDER}/{}.png`), and there is no engine to draw it: \
             give one, `imagine(\"…\", engine: \"imagine/local.py\")`, or set $ALEA_IMAGINE",
            stem(a)
        ));
    };
    let beside = a.dir.join(engine);
    let program = if beside.exists() { std::fs::canonicalize(&beside).unwrap_or(beside) } else { PathBuf::from(engine) };
    let mut child = std::process::Command::new(&program)
        .env("ALEA_SEED", a.seed.to_string())
        .env("ALEA_WIDTH", a.width.to_string())
        .env("ALEA_HEIGHT", a.height.to_string())
        .env("ALEA_MODEL", a.model)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("this picture hasn't been imagined yet (no `{FOLDER}/{}.png`), and alea cannot run the engine `{engine}` to draw it: {e}", stem(a)))?;
    let prompt = a.prompt.to_string();
    let mut stdin = child.stdin.take().expect("piped");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
    });
    let out = child.wait_with_output().map_err(|e| format!("the engine `{engine}` failed: {e}"))?;
    let _ = writer.join();
    let said = || {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.trim().lines().last().unwrap_or("").to_string();
        if last.is_empty() { String::new() } else { format!(": {last}") }
    };
    if !out.status.success() || out.stdout.is_empty() {
        return Err(format!("the engine `{engine}` gave no picture{}", said()));
    }
    let Some(kind) = kind_of(&out.stdout) else {
        return Err(format!("the engine `{engine}` did not write a PNG, a JPEG or a WebP{}", said()));
    };
    let folder = a.dir.join(FOLDER);
    let name = format!("{}.{kind}", stem(a));
    std::fs::create_dir_all(&folder)
        .and_then(|_| std::fs::write(folder.join(&name), &out.stdout))
        .map_err(|e| format!("cannot keep the picture in `{FOLDER}/{name}`: {e}"))?;
    Ok((folder.join(&name), format!("{FOLDER}/{name}")))
}

#[cfg(target_arch = "wasm32")]
pub fn picture(a: &Ask) -> Result<(PathBuf, String), String> {
    saved(a).ok_or_else(|| {
        format!(
            "this picture hasn't been imagined yet: in the browser, `imagine` shows the pictures already kept in \
             `{FOLDER}/` (here `{FOLDER}/{}.png`); render the work once on the command line",
            stem(a)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_says_what_was_asked() {
        let dir = Path::new(".");
        let ask = |prompt, seed, width| Ask { dir, prompt, engine: None, seed, width, height: 1024, model: "" };
        let a = stem(&ask("A rainy street, at night!", 7, 1024));
        assert!(a.starts_with("a-rainy-street-at-night-"), "{a}");
        assert_eq!(a, stem(&ask("A rainy street, at night!", 7, 1024)));
        assert_ne!(a, stem(&ask("A rainy street, at night!", 8, 1024)));
        assert_ne!(a, stem(&ask("A rainy street, at night!", 7, 512)));
        assert_eq!(stem(&ask("…", 1, 64)).len(), 8);
    }
}
