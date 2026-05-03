//! Compile side of slangmake: invoke Slang to produce bytecode + reflection,
//! either one permutation at a time ([`Compiler`]) or in bulk
//! ([`BatchCompiler`]).
//!
//! Gated by the `compiler` feature, which transitively enables `runtime`.

mod batch;
#[allow(clippy::module_inception)]
mod compiler;

pub use batch::{BatchCompiler, BatchOutput};
pub use compiler::{CompileResult, Compiler};
