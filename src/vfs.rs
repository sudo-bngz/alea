//! File access for the language. On the command line this is the disk. In the
//! browser there is no disk, so files (works, pictures, fonts) are handed over
//! from JavaScript and kept in memory under absolute paths like `/moon.alea`.
//! The native side also records which files a run read, so `--web` can bundle them.

use std::collections::{BTreeSet, HashMap};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{LazyLock, Mutex};

static MEMORY: LazyLock<Mutex<Option<HashMap<PathBuf, Vec<u8>>>>> = LazyLock::new(Default::default);
static TOUCHED: LazyLock<Mutex<BTreeSet<PathBuf>>> = LazyLock::new(Default::default);

/// Switches to in-memory files (used by the browser build).
pub fn use_memory() {
    let mut m = MEMORY.lock().unwrap();
    if m.is_none() {
        *m = Some(HashMap::new());
    }
}

pub fn put(path: &str, data: Vec<u8>) {
    use_memory();
    MEMORY.lock().unwrap().as_mut().unwrap().insert(normalize(Path::new(path)), data);
}

pub fn clear() {
    if let Some(m) = MEMORY.lock().unwrap().as_mut() {
        m.clear();
    }
}

/// Resolves `.` and `..` without touching any disk, from the root `/`.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::from("/");
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(n) => out.push(n),
            _ => {}
        }
    }
    out
}

pub fn canonicalize(p: &Path) -> io::Result<PathBuf> {
    match MEMORY.lock().unwrap().as_ref() {
        Some(m) => {
            let n = normalize(p);
            if m.contains_key(&n) { Ok(n) } else { Err(io::Error::from(io::ErrorKind::NotFound)) }
        }
        None => std::fs::canonicalize(p),
    }
}

pub fn read(p: &Path) -> io::Result<Vec<u8>> {
    match MEMORY.lock().unwrap().as_ref() {
        Some(m) => m.get(&normalize(p)).cloned().ok_or_else(|| io::Error::from(io::ErrorKind::NotFound)),
        None => {
            let data = std::fs::read(p)?;
            if let Ok(c) = std::fs::canonicalize(p) {
                TOUCHED.lock().unwrap().insert(c);
            }
            Ok(data)
        }
    }
}

pub fn read_to_string(p: &Path) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not valid UTF-8 text"))
}

/// The names of the files (not folders) in a folder, in no particular order.
pub fn list(dir: &Path) -> io::Result<Vec<String>> {
    match MEMORY.lock().unwrap().as_ref() {
        Some(m) => {
            let d = normalize(dir);
            Ok(m.keys().filter(|k| k.parent() == Some(d.as_path())).filter_map(|k| k.file_name()).map(|n| n.to_string_lossy().into_owned()).collect())
        }
        None => {
            let mut out = Vec::new();
            for e in std::fs::read_dir(dir)? {
                let e = e?;
                if e.file_type()?.is_file() {
                    out.push(e.file_name().to_string_lossy().into_owned());
                }
            }
            Ok(out)
        }
    }
}

/// Records a file as used without reading it (a file listed by `files()`
/// may be opened by another edition, so `--web` bundles it too).
pub fn touch(p: &Path) {
    if MEMORY.lock().unwrap().is_none() {
        if let Ok(c) = std::fs::canonicalize(p) {
            TOUCHED.lock().unwrap().insert(c);
        }
    }
}

/// Every file read from disk so far (native only), as canonical paths.
pub fn touched() -> Vec<PathBuf> {
    TOUCHED.lock().unwrap().iter().cloned().collect()
}
