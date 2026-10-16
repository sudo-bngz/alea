//! `alea work.alea --web page.html`: one self-contained page that draws a new
//! edition of the work on every visit. It embeds the WebAssembly engine, the
//! work, every work it `use`s, and the pictures and fonts they name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use alea::lexer::{self, Tok};
use alea::library::Module;

#[cfg(alea_wasm)]
const WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/web/dist/alea.wasm"));
#[cfg(not(alea_wasm))]
const WASM: &[u8] = &[];

const VIEWER: &str = include_str!("../web/viewer.html");
const ENGINE: &str = include_str!("../web/engine.js");

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"), // keeps `</script>` out of the page
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Every file the work needs: its sources, and any file a string in them names.
fn collect(m: &Module, out: &mut BTreeSet<PathBuf>) {
    if let Ok(p) = std::fs::canonicalize(&m.file) {
        if !out.insert(p) {
            return;
        }
    }
    if let Ok(toks) = lexer::lex(&m.src) {
        for t in toks {
            if let Tok::Str(s) = t.tok {
                if let Ok(p) = std::fs::canonicalize(m.dir.join(&s)) {
                    if p.is_file() {
                        out.insert(p);
                    }
                }
            }
        }
    }
    for d in m.deps.values() {
        collect(d, out);
    }
}

fn common_dir(paths: &[PathBuf]) -> PathBuf {
    let mut base = paths[0].parent().unwrap_or(Path::new("/")).to_path_buf();
    while !paths.iter().all(|p| p.starts_with(&base)) {
        if !base.pop() {
            break;
        }
    }
    base
}

pub fn export(module: &Module, out: &Path) -> Result<(), String> {
    if WASM.is_empty() {
        return Err("this build of alea has no browser engine: run `web/build.sh`, then rebuild alea\n".into());
    }
    let mut files = BTreeSet::new();
    collect(module, &mut files);
    // Pictures and fonts read while drawing (e.g. inside a `choose`).
    files.extend(alea::vfs::touched());
    let main = std::fs::canonicalize(&module.file).map_err(|e| format!("{}: {e}\n", module.file))?;
    let list: Vec<PathBuf> = files.into_iter().collect();
    let root = common_dir(&list);
    let vpath = |p: &Path| format!("/{}", p.strip_prefix(&root).unwrap_or(p).display());

    let mut entries = Vec::new();
    for p in &list {
        let data = std::fs::read(p).map_err(|e| format!("reading {}: {e}\n", p.display()))?;
        entries.push(format!(r#"{{"path":{},"data":"{}"}}"#, json_str(&vpath(p)), base64(&data)));
    }
    let title: String = module.work.name.chars().filter(|c| !"<>&\"".contains(*c)).collect();
    let page = VIEWER
        .replace("{{TITLE}}", &title)
        .replace("{{MAIN}}", &vpath(&main))
        .replace("/*ENGINE*/", ENGINE)
        .replace("/*FILES*/", &format!("[{}]", entries.join(",")))
        .replace("{{WASM}}", &base64(WASM));
    std::fs::write(out, page).map_err(|e| format!("writing {}: {e}\n", out.display()))?;
    println!("`{}` — web page with {} file(s): {}", module.work.name, list.len(), out.display());
    Ok(())
}
