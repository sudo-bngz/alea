//! Splits the source into tokens. Identifiers may contain any Unicode letter.

use crate::error::{Error, Res, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    Tilde,
    DoubleTilde,
    Pipe,
    LBracket,
    RBracket,
    Dot,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
    /// True when a newline precedes this token.
    pub newline_before: bool,
}

pub fn lex(src: &str) -> Res<Vec<Token>> {
    let cs: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut col) = (0usize, 1usize, 1usize);
    let mut nl = true;
    let mut toks = Vec::new();

    while i < cs.len() {
        let c = cs[i];
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            nl = true;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }
        if c == '#' || (c == '/' && cs.get(i + 1) == Some(&'/')) {
            while i < cs.len() && cs[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }

        let start = i;
        let start_col = col;
        let tok = if c.is_ascii_digit() {
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '_') {
                i += 1;
            }
            if i < cs.len() && cs[i] == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = cs[start..i].iter().filter(|c| **c != '_').collect();
            let n = s.parse().map_err(|_| {
                Error::new(format!("invalid number `{s}`"), Span { line, col: start_col, len: i - start })
            })?;
            Tok::Num(n)
        } else if c.is_alphabetic() || c == '_' {
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            Tok::Ident(cs[start..i].iter().collect())
        } else if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                match cs.get(i) {
                    None | Some('\n') => {
                        return Err(Error::new("unterminated string (missing `\"`)", Span { line, col: start_col, len: 1 }));
                    }
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some(&ch) => {
                        s.push(ch);
                        i += 1;
                    }
                }
            }
            Tok::Str(s)
        } else {
            let next = cs.get(i + 1).copied();
            let (t, n) = match (c, next) {
                ('(', _) => (Tok::LParen, 1),
                (')', _) => (Tok::RParen, 1),
                ('{', _) => (Tok::LBrace, 1),
                ('}', _) => (Tok::RBrace, 1),
                (',', _) => (Tok::Comma, 1),
                (':', _) => (Tok::Colon, 1),
                ('+', _) => (Tok::Plus, 1),
                ('-', _) => (Tok::Minus, 1),
                ('*', _) => (Tok::Star, 1),
                ('/', _) => (Tok::Slash, 1),
                ('%', _) => (Tok::Percent, 1),
                ('^', _) => (Tok::Caret, 1),
                ('|', _) => (Tok::Pipe, 1),
                ('[', _) => (Tok::LBracket, 1),
                (']', _) => (Tok::RBracket, 1),
                ('.', _) => (Tok::Dot, 1),
                ('~', Some('~')) => (Tok::DoubleTilde, 2),
                ('~', _) => (Tok::Tilde, 1),
                ('<', Some('=')) => (Tok::Le, 2),
                ('<', _) => (Tok::Lt, 1),
                ('>', Some('=')) => (Tok::Ge, 2),
                ('>', _) => (Tok::Gt, 1),
                ('=', Some('=')) => (Tok::EqEq, 2),
                ('=', _) => (Tok::Assign, 1),
                ('!', Some('=')) => (Tok::Ne, 2),
                _ => {
                    return Err(Error::new(format!("unexpected character `{c}`"), Span { line, col, len: 1 }));
                }
            };
            i += n;
            t
        };

        let len = i - start;
        col += len;
        toks.push(Token { tok, span: Span { line, col: start_col, len }, newline_before: nl });
        nl = false;
    }

    toks.push(Token { tok: Tok::Eof, span: Span { line, col, len: 1 }, newline_before: true });
    Ok(toks)
}
