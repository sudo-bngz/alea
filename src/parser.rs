//! Parser: recursive descent for statements, Pratt (binding power) for expressions.
//!
//! Grammar:
//!   program   := ('use' "file.alea")* 'work' Name params? '{' stmt* '}'
//!   params    := '(' (name ('=' expr)?),* ')'
//!   stmt      := 'let' name '=' expr
//!              | 'let' name params '=' expr                 (a function)
//!              | 'field' name '=' expr
//!              | 'pattern' name '=' 'repeat' expr 'event' '(' args ')'
//!              | 'constraint' expr
//!              | 'render' kind '(' args ')'
//!              | 'output' expr
//!              | 'grow' ('over' expr)? ('for' expr)? '{' (name '=' expr 'then' expr)* '}'
//!   args      := ( (name ':')? expr ),*
//!   expr      := Pratt over: or < and < comparisons < + - < * / % < ^ < unary (- ~ ~~)
//!   primary   := number | "string" | name | name '(' args ')' | '(' expr ')'
//!              | '[' (expr (',' expr)*)? ']' | '[' expr ('for' name 'in' expr)+ ']'
//!   postfix   := primary ( '[' expr ']' | '.' name )*
//!              | 'choose' '{' (weight ':')? expr ('|' (weight ':')? expr)* '}'

use crate::ast::*;
use crate::error::{Error, Res, Span};
use crate::lexer::{Tok, Token};

const KEYWORDS: &[&str] =
    &["use", "work", "let", "field", "pattern", "constraint", "render", "output", "repeat", "event", "choose", "and", "or", "grow", "then"];
const BUILTIN_NAMES: &[&str] = &["x", "y", "t", "i", "n", "pi", "tau"];

pub fn parse(toks: Vec<Token>) -> Res<Program> {
    Parser { toks, pos: 0 }.program()
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

fn describe(t: &Tok) -> String {
    let s = match t {
        Tok::Ident(s) => return format!("`{s}`"),
        Tok::Num(n) => return format!("the number {n}"),
        Tok::Str(s) => return format!("the string \"{s}\""),
        Tok::Eof => return "end of file".into(),
        Tok::LParen => "(",
        Tok::RParen => ")",
        Tok::LBrace => "{",
        Tok::RBrace => "}",
        Tok::Comma => ",",
        Tok::Colon => ":",
        Tok::Assign => "=",
        Tok::Plus => "+",
        Tok::Minus => "-",
        Tok::Star => "*",
        Tok::Slash => "/",
        Tok::Percent => "%",
        Tok::Caret => "^",
        Tok::Lt => "<",
        Tok::Gt => ">",
        Tok::Le => "<=",
        Tok::Ge => ">=",
        Tok::EqEq => "==",
        Tok::Ne => "!=",
        Tok::Tilde => "~",
        Tok::DoubleTilde => "~~",
        Tok::Pipe => "|",
        Tok::LBracket => "[",
        Tok::RBracket => "]",
        Tok::Dot => ".",
    };
    format!("`{s}`")
}

impl Parser {
    fn cur(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn peek_next(&self) -> &Token {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)]
    }

    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn at(&self, t: &Tok) -> bool {
        &self.cur().tok == t
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(&self.cur().tok, Tok::Ident(s) if s == kw)
    }

    fn error_here(&self, msg: impl Into<String>) -> Error {
        Error::new(msg, self.cur().span)
    }

    fn expect(&mut self, t: Tok, what: &str) -> Res<Token> {
        if self.at(&t) {
            Ok(self.bump())
        } else {
            Err(self.error_here(format!("expected {what}, found {}", describe(&self.cur().tok))))
        }
    }

    fn expect_kw(&mut self, kw: &str, context: &str) -> Res<Span> {
        if self.at_kw(kw) {
            Ok(self.bump().span)
        } else {
            Err(self.error_here(format!("expected `{kw}` {context}, found {}", describe(&self.cur().tok))))
        }
    }

    fn ident(&mut self, what: &str) -> Res<(String, Span)> {
        match &self.cur().tok {
            Tok::Ident(s) => {
                let s = s.clone();
                Ok((s, self.bump().span))
            }
            t => Err(self.error_here(format!("expected {what}, found {}", describe(t)))),
        }
    }

    fn free_name(&mut self) -> Res<(String, Span)> {
        let (name, span) = self.ident("a name")?;
        if KEYWORDS.contains(&name.as_str()) || BUILTIN_NAMES.contains(&name.as_str()) {
            return Err(Error::new(format!("`{name}` is reserved by the language"), span));
        }
        Ok((name, span))
    }

    fn program(&mut self) -> Res<Program> {
        let mut uses = Vec::new();
        while self.at_kw("use") {
            self.bump();
            let t = self.bump();
            match t.tok {
                Tok::Str(path) => uses.push(Use { path, span: t.span }),
                other => {
                    return Err(Error::new(
                        format!("`use` expects a file name in quotes, like `use \"moon.alea\"`, found {}", describe(&other)),
                        t.span,
                    ));
                }
            }
        }
        if !self.at_kw("work") {
            let msg = if uses.is_empty() {
                "a program starts with `work Name { … }`"
            } else {
                "after the `use` lines, a program continues with `work Name { … }`"
            };
            return Err(self.error_here(msg));
        }
        self.bump();
        let (name, _) = self.ident("the name of the work")?;
        let params = if self.at(&Tok::LParen) { self.params()? } else { Vec::new() };
        self.expect(Tok::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        while !self.at(&Tok::RBrace) {
            if self.at(&Tok::Eof) {
                return Err(self.error_here("missing `}` to close the work"));
            }
            stmts.push(self.stmt()?);
        }
        self.bump();
        if !self.at(&Tok::Eof) {
            return Err(self.error_here("nothing is expected after the end of the work (one work per file)"));
        }
        Ok(Program { uses, work: Work { name, params, stmts } })
    }

    /// `(size = 0.3, count)` after a work's name.
    fn params(&mut self) -> Res<Vec<Param>> {
        self.bump(); // (
        let mut params: Vec<Param> = Vec::new();
        while !self.at(&Tok::RParen) {
            let (name, span) = self.free_name()?;
            if params.iter().any(|p| p.name == name) {
                return Err(Error::new(format!("parameter `{name}` declared twice"), span));
            }
            let default = if self.at(&Tok::Assign) {
                self.bump();
                Some(self.expr(0)?)
            } else {
                None
            };
            params.push(Param { name, default, span });
            if self.at(&Tok::Comma) {
                self.bump();
            } else if !self.at(&Tok::RParen) {
                return Err(self.error_here(format!("expected `,` or `)`, found {}", describe(&self.cur().tok))));
            }
        }
        self.bump(); // )
        Ok(params)
    }

    fn stmt(&mut self) -> Res<Stmt> {
        let tok = self.cur().clone();
        let Tok::Ident(kw) = &tok.tok else {
            return Err(self.error_here(format!(
                "expected a statement (let, field, pattern, grow, constraint, render, output), found {}",
                describe(&tok.tok)
            )));
        };
        match kw.as_str() {
            "let" if matches!(self.peek_next().tok, Tok::Ident(_))
                && self.toks.get(self.pos + 2).is_some_and(|t| t.tok == Tok::LParen && !t.newline_before) =>
            {
                self.bump();
                let (name, span) = self.free_name()?;
                let params = self.params()?;
                self.expect(Tok::Assign, "`=` (a function reads `let name(a, b) = …`)")?;
                let body = self.expr(0)?;
                Ok(Stmt::Func { name, params, body, span })
            }
            "let" | "field" => {
                self.bump();
                let (name, span) = self.free_name()?;
                self.expect(Tok::Assign, "`=`")?;
                let val = self.expr(0)?;
                Ok(if kw == "let" { Stmt::Let { name, val, span } } else { Stmt::Field { name, val, span } })
            }
            "pattern" => {
                self.bump();
                let (name, span) = self.free_name()?;
                self.expect(Tok::Assign, "`=`")?;
                self.expect_kw("repeat", "(a pattern reads `pattern p = repeat N event(…)`)")?;
                let count = self.expr(0)?;
                self.expect_kw("event", "after the repeat count")?;
                self.expect(Tok::LParen, "`(`")?;
                let attrs = self.args()?;
                Ok(Stmt::Pattern { name, count, attrs, span })
            }
            "constraint" => {
                self.bump();
                let val = self.expr(0)?;
                Ok(Stmt::Constraint { span: val.span, val })
            }
            "output" => {
                let span = self.bump().span;
                let val = self.expr(0)?;
                Ok(Stmt::Output { val, span })
            }
            "render" => {
                self.bump();
                let (kind, span) = self.ident("a render kind (image, sound, video, sculpture)")?;
                self.expect(Tok::LParen, "`(`")?;
                let args = self.args()?;
                Ok(Stmt::Render { kind, args, span })
            }
            "grow" => {
                let span = self.bump().span;
                let size = if self.at_kw("over") {
                    self.bump();
                    Some(self.expr(0)?)
                } else {
                    None
                };
                let steps = if self.at_kw("for") {
                    self.bump();
                    Some(self.expr(0)?)
                } else {
                    None
                };
                self.expect(Tok::LBrace, "`{` (a grow block reads `grow over 128 for 200 { cells = … then … }`)")?;
                let mut rules = Vec::new();
                while !self.at(&Tok::RBrace) {
                    if self.at(&Tok::Eof) {
                        return Err(self.error_here("this `grow` block is not closed: `}` is missing"));
                    }
                    let (name, span) = self.free_name()?;
                    self.expect(Tok::Assign, "`=` (each line reads `name = start then next`)")?;
                    let start = self.expr(0)?;
                    self.expect_kw("then", "after the starting value (each line reads `name = start then next`)")?;
                    let next = self.expr(0)?;
                    rules.push(GrowRule { name, start, next, span });
                }
                self.bump(); // }
                if rules.is_empty() {
                    return Err(Error::new("a `grow` block needs at least one line: `name = start then next`", span));
                }
                Ok(Stmt::Grow { size, steps, rules, span })
            }
            _ => Err(Error::new(
                format!("unknown statement `{kw}` — expected one of: let, field, pattern, grow, constraint, render, output"),
                tok.span,
            )),
        }
    }

    /// Arguments after `(`, up to and including `)`.
    fn args(&mut self) -> Res<Vec<Arg>> {
        let mut v = Vec::new();
        if self.at(&Tok::RParen) {
            self.bump();
            return Ok(v);
        }
        loop {
            let candidate = match &self.cur().tok {
                Tok::Ident(n) => Some(n.clone()),
                _ => None,
            };
            let name = match candidate {
                Some(n) if self.peek_next().tok == Tok::Colon => {
                    self.bump();
                    self.bump();
                    Some(n)
                }
                _ => None,
            };
            let val = self.expr(0)?;
            v.push(Arg { name, val });
            if self.at(&Tok::Comma) {
                self.bump();
                if self.at(&Tok::RParen) {
                    self.bump();
                    break;
                }
                continue;
            }
            self.expect(Tok::RParen, "`,` or `)`")?;
            break;
        }
        Ok(v)
    }

    fn expr(&mut self, min_bp: u8) -> Res<Expr> {
        let first = self.prefix()?;
        let mut lhs = self.postfix(first)?;
        loop {
            let (op, l_bp, r_bp) = match &self.cur().tok {
                Tok::Ident(k) if k == "or" => (BinOp::Or, 1, 2),
                Tok::Ident(k) if k == "and" => (BinOp::And, 3, 4),
                Tok::Lt => (BinOp::Lt, 5, 6),
                Tok::Gt => (BinOp::Gt, 5, 6),
                Tok::Le => (BinOp::Le, 5, 6),
                Tok::Ge => (BinOp::Ge, 5, 6),
                Tok::EqEq => (BinOp::Eq, 5, 6),
                Tok::Ne => (BinOp::Ne, 5, 6),
                Tok::Plus => (BinOp::Add, 7, 8),
                Tok::Minus => (BinOp::Sub, 7, 8),
                Tok::Star => (BinOp::Mul, 9, 10),
                Tok::Slash => (BinOp::Div, 9, 10),
                Tok::Percent => (BinOp::Mod, 9, 10),
                Tok::Caret => (BinOp::Pow, 12, 11), // right-associative
                _ => break,
            };
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr(r_bp)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Bin(op, Box::new(lhs), Box::new(rhs)), span };
        }
        Ok(lhs)
    }

    /// `list[i]` and `point.x`, as many as follow.
    fn postfix(&mut self, mut e: Expr) -> Res<Expr> {
        loop {
            if self.at(&Tok::LBracket) && !self.cur().newline_before {
                self.bump();
                let i = self.expr(0)?;
                let end = self.expect(Tok::RBracket, "`]`")?;
                e = Expr { span: e.span.to(end.span), kind: ExprKind::Index(Box::new(e), Box::new(i)) };
            } else if self.at(&Tok::Dot) {
                self.bump();
                let (name, end) = self.ident("a name after `.` (such as `x` or `y`)")?;
                e = Expr { span: e.span.to(end), kind: ExprKind::Member(Box::new(e), name) };
            } else {
                return Ok(e);
            }
        }
    }

    /// After `[`: a list, or a list built by a loop.
    fn list(&mut self, start: Span) -> Res<Expr> {
        if self.at(&Tok::RBracket) {
            let end = self.bump().span;
            return Ok(Expr { kind: ExprKind::List(Vec::new()), span: start.to(end) });
        }
        let first = self.expr(0)?;
        if self.at_kw("for") {
            let mut loops = Vec::new();
            while self.at_kw("for") {
                self.bump();
                let (name, _) = self.free_name()?;
                self.expect_kw("in", "after the loop's name (`[… for p in points]`)")?;
                loops.push((name, self.expr(0)?));
            }
            let end = self.expect(Tok::RBracket, "`]` or another `for`")?;
            return Ok(Expr { kind: ExprKind::Comprehension { body: Box::new(first), loops }, span: start.to(end.span) });
        }
        let mut items = vec![first];
        loop {
            if self.at(&Tok::Comma) {
                self.bump();
                if self.at(&Tok::RBracket) {
                    continue;
                }
                items.push(self.expr(0)?);
                continue;
            }
            let end = self.expect(Tok::RBracket, "`,` or `]`")?;
            return Ok(Expr { kind: ExprKind::List(items), span: start.to(end.span) });
        }
    }

    /// A text with `{value}` or `{value:02}` inside.
    fn template(&mut self, s: &str, span: Span) -> Res<Expr> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut rest = s;
        while let Some(open) = rest.find('{') {
            lit.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else {
                return Err(Error::new("a `{` in this text is not closed by `}`", span));
            };
            let inner = &rest[open + 1..open + close];
            let (code, pad) = match inner.rsplit_once(':') {
                Some((c, w)) if !w.is_empty() && w.chars().all(|ch| ch.is_ascii_digit()) => (c, w.parse().unwrap_or(0)),
                _ => (inner, 0),
            };
            let toks = crate::lexer::lex(code).map_err(|e| Error::new(format!("in `{{{inner}}}`: {}", e.msg), span))?;
            let mut sub = Parser { toks, pos: 0 };
            let mut e = sub.expr(0).map_err(|e| Error::new(format!("in `{{{inner}}}`: {}", e.msg), span))?;
            if !sub.at(&Tok::Eof) {
                return Err(Error::new(format!("in `{{{inner}}}`: unexpected {}", describe(&sub.cur().tok)), span));
            }
            relocate(&mut e, span);
            if !lit.is_empty() {
                parts.push(TextPart::Lit(std::mem::take(&mut lit)));
            }
            parts.push(TextPart::Value(e, pad));
            rest = &rest[open + close + 1..];
        }
        lit.push_str(rest);
        if !lit.is_empty() {
            parts.push(TextPart::Lit(lit));
        }
        Ok(Expr { kind: ExprKind::Template(parts), span })
    }

    fn prefix(&mut self) -> Res<Expr> {
        let t = self.bump();
        let span = t.span;
        match t.tok {
            Tok::Num(n) => Ok(Expr { kind: ExprKind::Num(n), span }),
            Tok::Str(s) if s.contains('{') => self.template(&s, span),
            Tok::Str(s) => Ok(Expr { kind: ExprKind::Str(s), span }),
            Tok::LBracket => self.list(span),
            Tok::Minus => {
                let e = self.expr(11)?;
                Ok(Expr { span: span.to(e.span), kind: ExprKind::Neg(Box::new(e)) })
            }
            Tok::Tilde | Tok::DoubleTilde => {
                let per_point = t.tok == Tok::DoubleTilde;
                let e = self.expr(13)?;
                Ok(Expr { span: span.to(e.span), kind: ExprKind::Draw { dist: Box::new(e), per_point } })
            }
            Tok::LParen => {
                let e = self.expr(0)?;
                self.expect(Tok::RParen, "`)`")?;
                Ok(e)
            }
            Tok::Ident(name) => {
                if name == "choose" && self.at(&Tok::LBrace) {
                    return self.choose(span);
                }
                if name == "synth" && self.at(&Tok::LBrace) {
                    return self.synth(span);
                }
                if self.at(&Tok::LParen) && !self.cur().newline_before {
                    self.bump();
                    let args = self.args()?;
                    let end = self.prev_span();
                    return Ok(Expr { kind: ExprKind::Call(name, args), span: span.to(end) });
                }
                if KEYWORDS.contains(&name.as_str()) && name != "output" {
                    return Err(Error::new(format!("`{name}` is a keyword, not a value"), span));
                }
                Ok(Expr { kind: ExprKind::Var(name), span })
            }
            other => Err(Error::new(format!("expected an expression, found {}", describe(&other)), span)),
        }
    }

    /// `synth { let env = adsr() … sine(note.freq) * env }`: some `let`s, then
    /// the sound, as an expression.
    fn synth(&mut self, start: Span) -> Res<Expr> {
        self.bump(); // {
        let mut lets = Vec::new();
        while self.at_kw("let") {
            self.bump();
            let (name, _) = self.free_name()?;
            self.expect(Tok::Assign, "`=`")?;
            lets.push((name, self.expr(0)?));
        }
        if self.at(&Tok::RBrace) {
            return Err(self.error_here("a synth ends with its sound: an expression such as `sine(note.freq) * adsr()`"));
        }
        let body = self.expr(0)?;
        let end = self.expect(Tok::RBrace, "`}` (a synth is some `let`s, then one expression: its sound)")?;
        Ok(Expr { kind: ExprKind::Synth { lets, body: Box::new(body) }, span: start.to(end.span) })
    }

    fn choose(&mut self, start: Span) -> Res<Expr> {
        self.bump(); // {
        let mut branches = Vec::new();
        loop {
            let a = self.expr(0)?;
            let branch = if self.at(&Tok::Colon) {
                self.bump();
                (a, self.expr(0)?)
            } else {
                (Expr { kind: ExprKind::Num(1.0), span: a.span }, a)
            };
            branches.push(branch);
            if self.at(&Tok::Pipe) {
                self.bump();
                continue;
            }
            let end = self.expect(Tok::RBrace, "`|` or `}`")?;
            return Ok(Expr { kind: ExprKind::Choose(branches), span: start.to(end.span) });
        }
    }
}

/// Errors inside a `{…}` of a text point at the text itself.
fn relocate(e: &mut Expr, span: Span) {
    e.span = span;
    match &mut e.kind {
        ExprKind::Neg(a) | ExprKind::Member(a, _) => relocate(a, span),
        ExprKind::Bin(_, a, b) | ExprKind::Index(a, b) => {
            relocate(a, span);
            relocate(b, span);
        }
        ExprKind::Call(_, args) => args.iter_mut().for_each(|a| relocate(&mut a.val, span)),
        ExprKind::Draw { dist, .. } => relocate(dist, span),
        ExprKind::Choose(bs) => bs.iter_mut().for_each(|(w, v)| {
            relocate(w, span);
            relocate(v, span);
        }),
        ExprKind::List(items) => items.iter_mut().for_each(|i| relocate(i, span)),
        ExprKind::Comprehension { body, loops } => {
            relocate(body, span);
            loops.iter_mut().for_each(|(_, l)| relocate(l, span));
        }
        ExprKind::Synth { lets, body } => {
            relocate(body, span);
            lets.iter_mut().for_each(|(_, l)| relocate(l, span));
        }
        _ => {}
    }
}
