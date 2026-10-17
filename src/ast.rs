//! Syntax tree: what the artist wrote, before any draw.

use crate::error::Span;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Num(f64),
    Str(String),
    Var(String),
    Neg(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Arg>),
    /// `~d` (drawn once, where the expression is evaluated) or `~~d` (redrawn at every point).
    Draw { dist: Box<Expr>, per_point: bool },
    /// `choose { weight: expr | … }`
    Choose(Vec<(Expr, Expr)>),
    /// `[a, b, c]`
    List(Vec<Expr>),
    /// `[body for name in list for other in list2]`
    Comprehension { body: Box<Expr>, loops: Vec<(String, Expr)> },
    /// `list[i]`
    Index(Box<Expr>, Box<Expr>),
    /// `point.x`, `point.y`
    Member(Box<Expr>, String),
    /// `"{h:02}:00"`: a text with values inside.
    Template(Vec<TextPart>),
    /// `synth { let a = … ; expression }`: a patch, played once per note.
    Synth { lets: Vec<(String, Expr)>, body: Box<Expr> },
}

#[derive(Clone, Debug)]
pub enum TextPart {
    Lit(String),
    /// A value, and the width to pad it to with zeros (0: none).
    Value(Expr, usize),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub name: Option<String>,
    pub val: Expr,
}

#[allow(dead_code)] // spans kept for future diagnostics
#[derive(Clone, Debug)]
pub enum Stmt {
    Let { name: String, val: Expr, span: Span },
    /// `let name(a, b = 1) = expr`: a function.
    Func { name: String, params: Vec<Param>, body: Expr, span: Span },
    Field { name: String, val: Expr, span: Span },
    Pattern { name: String, count: Expr, attrs: Vec<Arg>, span: Span },
    Constraint { val: Expr, span: Span },
    Render { kind: String, args: Vec<Arg>, span: Span },
    /// What the work gives when another work calls it.
    Output { val: Expr, span: Span },
    /// `grow over N for S { name = start then next … }`: fields that evolve
    /// generation by generation on an N×N grid, each from the one before.
    Grow { size: Option<Expr>, steps: Option<Expr>, rules: Vec<GrowRule>, span: Span },
}

/// One field of a `grow` block: where it starts, and how it becomes the next.
#[derive(Clone, Debug)]
pub struct GrowRule {
    pub name: String,
    pub start: Expr,
    pub next: Expr,
    pub span: Span,
}

/// `work Name(size = 0.3, count)`: a parameter, with an optional default.
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub span: Span,
}

/// `use "moon.alea"`
#[derive(Clone, Debug)]
pub struct Use {
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub uses: Vec<Use>,
    pub work: Work,
}

#[derive(Clone, Debug)]
pub struct Work {
    pub name: String,
    pub params: Vec<Param>,
    pub stmts: Vec<Stmt>,
}
