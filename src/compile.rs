//! Tile programs: a field compiled to a flat list of instructions.
//!
//! After a tile has been simplified (bounds.rs), what is left is still a tree,
//! and walking a tree at every pixel means following pointers, re-reading the
//! same shared parts and checking a memo for each of them. Here the tree is
//! turned once per tile into a straight program over numbered slots: every
//! part shared in the tree is computed once per pixel, constants are set once
//! per tile, and the pixel loop is a single pass over the instructions.
//!
//! The arithmetic is the same functions as `Field::eval`, in the same order
//! for each value, so the pixels are byte for byte the same.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::BinOp;
use crate::eval::{apply_bin_pub as apply_bin, apply_builtin_pub as apply_builtin, grain_value, Builtin, Dist, Field};

/// The slots every program starts with.
const X: usize = 0;
const Y: usize = 1;
const T: usize = 2;

enum Op {
    Neg(usize, usize),
    Bin(BinOp, usize, usize, usize),
    Call(Builtin, usize, [usize; 4], u8, u64),
    Grain(usize, Dist, u64, usize, usize, usize, u64),
    /// A cell of a grow grid (the previous generation), offset by (di, dj).
    Grid(usize, Arc<Vec<f64>>, usize, i64, i64, usize, usize),
    /// Anything else (a pattern, a picture, a grid, a cache…): evaluated as
    /// a field, at the point held in two slots.
    Eval(usize, Arc<Field>, usize, usize, usize, u64),
}

pub struct Program {
    ops: Vec<Op>,
    /// Slot values set once per tile: the constants, then room for the rest.
    init: Vec<f64>,
    out: usize,
}

struct Compiler {
    ops: Vec<Op>,
    init: Vec<f64>,
    shared: HashMap<(*const Field, usize, usize, usize, u64), usize>,
}

impl Compiler {
    fn slot(&mut self) -> usize {
        self.init.push(0.0);
        self.init.len() - 1
    }

    fn constant(&mut self, v: f64) -> usize {
        let s = self.slot();
        self.init[s] = v;
        s
    }

    fn compile(&mut self, f: &Field, x: usize, y: usize, t: usize, seed: u64) -> usize {
        match f {
            Field::Const(c) => self.constant(*c),
            Field::X => x,
            Field::Y => y,
            Field::T => t,
            Field::Neg(a) => {
                let a = self.compile(a, x, y, t, seed);
                let d = self.slot();
                self.ops.push(Op::Neg(d, a));
                d
            }
            Field::Bin(op, a, b) => {
                let (a, b) = (self.compile(a, x, y, t, seed), self.compile(b, x, y, t, seed));
                let d = self.slot();
                self.ops.push(Op::Bin(*op, d, a, b));
                d
            }
            Field::Call(b, args) => {
                let mut s = [0usize; 4];
                for (k, a) in args.iter().enumerate() {
                    s[k] = self.compile(a, x, y, t, seed);
                }
                let d = self.slot();
                self.ops.push(Op::Call(*b, d, s, args.len() as u8, seed));
                d
            }
            Field::Grain { dist, id } => {
                let d = self.slot();
                self.ops.push(Op::Grain(d, dist.clone(), *id, x, y, t, seed));
                d
            }
            // A shared part: compiled once for each point and seed it is read at.
            Field::Ref(r) | Field::Memo { inner: r, .. } => {
                let key = (Arc::as_ptr(r), x, y, t, seed);
                if let Some(s) = self.shared.get(&key) {
                    return *s;
                }
                let s = self.compile(r, x, y, t, seed);
                self.shared.insert(key, s);
                s
            }
            Field::Remap { inner, x: fx, y: fy } => {
                let (nx, ny) = (self.compile(fx, x, y, t, seed), self.compile(fy, x, y, t, seed));
                let key = (Arc::as_ptr(inner), nx, ny, t, seed);
                if let Some(s) = self.shared.get(&key) {
                    return *s;
                }
                let s = self.compile(inner, nx, ny, t, seed);
                self.shared.insert(key, s);
                s
            }
            Field::Reseed { inner, seed: s } => self.compile(inner, x, y, t, *s),
            Field::Retime { inner, time } => {
                let nt = self.compile(time, x, y, t, seed);
                self.compile(inner, x, y, nt, seed)
            }
            Field::Grid { data, res, di, dj } => {
                let d = self.slot();
                self.ops.push(Op::Grid(d, data.clone(), *res, *di, *dj, x, y));
                d
            }
            other => {
                let d = self.slot();
                self.ops.push(Op::Eval(d, Arc::new(other.clone()), x, y, t, seed));
                d
            }
        }
    }
}

impl Program {
    pub fn new(f: &Field, seed: u64) -> Program {
        let mut c = Compiler { ops: Vec::new(), init: vec![0.0; 3], shared: HashMap::new() };
        let out = c.compile(f, X, Y, T, seed);
        Program { ops: c.ops, init: c.init, out }
    }

    /// A scratch space for running this program.
    pub fn slots(&self, t: f64) -> Vec<f64> {
        let mut s = self.init.clone();
        s[T] = t;
        s
    }

    /// The field's value at (x, y), using `s` (from `slots`) as scratch.
    #[inline]
    pub fn run(&self, s: &mut [f64], x: f64, y: f64) -> f64 {
        s[X] = x;
        s[Y] = y;
        for op in &self.ops {
            match op {
                Op::Neg(d, a) => s[*d] = -s[*a],
                Op::Bin(op, d, a, b) => s[*d] = apply_bin(*op, s[*a], s[*b]),
                Op::Call(b, d, args, n, seed) => {
                    let mut v = [0.0; 4];
                    for k in 0..*n as usize {
                        v[k] = s[args[k]];
                    }
                    s[*d] = apply_builtin(*b, &v[..*n as usize], *seed);
                }
                Op::Grain(d, dist, id, gx, gy, gt, seed) => s[*d] = grain_value(dist, *id, [s[*gx], s[*gy], s[*gt]], *seed),
                Op::Eval(d, f, ex, ey, et, seed) => s[*d] = f.eval([s[*ex], s[*ey], s[*et]], *seed),
                Op::Grid(d, data, res, di, dj, gx, gy) => {
                    // the same arithmetic as Field::eval for a grid
                    let n = *res as i64;
                    let cell = |v: f64| {
                        let t = v as i64;
                        if (t as f64) > v { t - 1 } else { t }
                    };
                    let i = (cell(s[*gx] * n as f64) + di).rem_euclid(n) as usize;
                    let j = (cell(s[*gy] * n as f64) + dj).rem_euclid(n) as usize;
                    s[*d] = data[j * *res + i];
                }
            }
        }
        s[self.out]
    }
}
