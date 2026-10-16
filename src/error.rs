//! Errors carrying a source position, printed rustc-style.

use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

impl Span {
    /// Extends this span to the end of `other` (when on the same line).
    pub fn to(self, other: Span) -> Span {
        if other.line == self.line && other.col + other.len >= self.col {
            Span { len: other.col + other.len - self.col, ..self }
        } else {
            self
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub span: Option<Span>,
    /// The file the span points into, when it is not the file being run
    /// (an error inside a work brought in with `use`).
    pub origin: Option<(String, Arc<str>)>,
}

pub type Res<T> = Result<T, Error>;

impl Error {
    pub fn new(msg: impl Into<String>, span: Span) -> Self {
        Error { msg: msg.into(), span: Some(span), origin: None }
    }

    pub fn bare(msg: impl Into<String>) -> Self {
        Error { msg: msg.into(), span: None, origin: None }
    }

    /// Attributes the error to `file`, unless it already points into another one.
    pub fn in_file(mut self, file: &str, src: &Arc<str>) -> Self {
        if self.origin.is_none() {
            self.origin = Some((file.to_string(), src.clone()));
        }
        self
    }

    pub fn display(&self, source: &str, file: &str) -> String {
        let (source, file) = match &self.origin {
            Some((f, s)) => (&**s, f.as_str()),
            None => (source, file),
        };
        let Some(s) = self.span else {
            return format!("error: {}\n", self.msg);
        };
        let text = source.lines().nth(s.line.saturating_sub(1)).unwrap_or("");
        let pad = " ".repeat(s.col.saturating_sub(1));
        let carets = "^".repeat(s.len.max(1));
        format!(
            "error: {msg}\n  --> {file}:{l}:{c}\n     |\n{l:>4} | {text}\n     | {pad}{carets}\n",
            msg = self.msg,
            l = s.line,
            c = s.col,
        )
    }
}
