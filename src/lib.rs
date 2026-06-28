//! pattern_engine — read example strings, emit the tightest pattern that
//! matches them and (given counter-examples) nothing else.
//!
//! Pipeline: read → tokenize → align → synthesize → verify → render. See
//! `README.md` for the product vision and `CLAUDE.md` for conventions.

pub mod align;
pub mod cli;
pub mod input;
pub mod ir;
pub mod json;
pub mod render;
pub mod synth;
pub mod tokenize;
pub mod verify;
