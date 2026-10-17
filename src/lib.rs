//! alea — a language whose programs are artworks: every run draws a new edition.
//!
//! The library is shared by the command line (`main.rs`, feature `native`) and
//! the browser build (`web.rs`, compiled to WebAssembly).

pub mod ast;
pub mod bounds;
pub mod compile;
pub mod error;
pub mod harmony;
pub mod input;
pub mod eval;
pub mod lexer;
pub mod library;
pub mod media;
pub mod midi;
pub mod paint;
pub mod parser;
pub mod prims;
pub mod raster;
pub mod sound;
pub mod vfs;

#[cfg(feature = "native")]
pub mod render;

#[cfg(feature = "native")]
pub mod live;

#[cfg(target_arch = "wasm32")]
mod web;
