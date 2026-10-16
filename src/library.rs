//! Loads a work and, through its `use` lines, every work it builds on.
//!
//! Each file only sees the works it `use`s itself: nothing leaks through from
//! a file's own imports. A file reached twice is loaded once, and a cycle
//! (`a` uses `b`, which uses `a`) is reported at the `use` that closes it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ast::Work;
use crate::error::{Error, Res};
use crate::{lexer, parser};

pub struct Module {
    pub work: Work,
    /// The file name as shown in error messages.
    pub file: String,
    pub src: Arc<str>,
    /// The folder of the file: `use`, `picture` and `font` paths are relative to it.
    pub dir: PathBuf,
    /// The works this file can call, by name.
    pub deps: HashMap<String, Arc<Module>>,
}

pub fn load(path: &str) -> Res<Arc<Module>> {
    let mut loader = Loader { loaded: HashMap::new(), stack: Vec::new() };
    let canon = crate::vfs::canonicalize(Path::new(path)).map_err(|e| Error::bare(format!("reading {path}: {e}")))?;
    loader.load(&canon, path.to_string())
}

struct Loader {
    loaded: HashMap<PathBuf, Arc<Module>>,
    stack: Vec<PathBuf>,
}

fn short(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

impl Loader {
    fn load(&mut self, canon: &Path, shown: String) -> Res<Arc<Module>> {
        if let Some(m) = self.loaded.get(canon) {
            return Ok(m.clone());
        }
        let text = crate::vfs::read_to_string(canon).map_err(|e| Error::bare(format!("reading {shown}: {e}")))?;
        let src: Arc<str> = Arc::from(text);
        let at = |e: Error| e.in_file(&shown, &src);

        let program = parser::parse(lexer::lex(&src).map_err(at)?).map_err(at)?;
        let dir = canon.parent().unwrap_or(Path::new("."));

        self.stack.push(canon.to_path_buf());
        let mut deps: HashMap<String, Arc<Module>> = HashMap::new();
        for u in &program.uses {
            let target = dir.join(&u.path);
            let child = crate::vfs::canonicalize(&target)
                .map_err(|e| at(Error::new(format!("cannot open `{}`: {e}", u.path), u.span)))?;
            if let Some(k) = self.stack.iter().position(|p| *p == child) {
                let mut chain: Vec<String> = self.stack[k..].iter().map(|p| short(p)).collect();
                chain.push(short(&child));
                return Err(at(Error::new(format!("circular `use`: {}", chain.join(" → ")), u.span)));
            }
            let shown_child = Path::new(&shown).parent().unwrap_or(Path::new("")).join(&u.path);
            let m = self.load(&child, shown_child.display().to_string())?;
            let name = m.work.name.clone();
            if name == program.work.name {
                return Err(at(Error::new(
                    format!("this file's work is also called `{name}`: rename one of them"),
                    u.span,
                )));
            }
            if let Some(prev) = deps.get(&name) {
                if !Arc::ptr_eq(prev, &m) {
                    return Err(at(Error::new(format!("two different works are called `{name}`"), u.span)));
                }
            }
            deps.insert(name, m);
        }
        self.stack.pop();

        let module = Arc::new(Module { work: program.work, file: shown, src, dir: dir.to_path_buf(), deps });
        self.loaded.insert(canon.to_path_buf(), module.clone());
        Ok(module)
    }
}
