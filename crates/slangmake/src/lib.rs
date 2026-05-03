//! Idiomatic Rust wrapper around [slangmake], a build tool that compiles
//! Slang shader permutations into a single `.bin` blob holding every
//! variant's bytecode and full reflection data.
//!
//! [slangmake]: https://github.com/EvanLuo42/slangmake
//!
//! # Features
//!
//! | Feature    | Module          | What it exposes                                   |
//! | ---------- | --------------- | ------------------------------------------------- |
//! | `runtime`  | [`runtime`]     | [`runtime::BlobReader`], [`runtime::BlobWriter`], [`runtime::Reflection`] |
//! | `compiler` | [`compiler`]    | [`compiler::Compiler`], [`compiler::BatchCompiler`] |
//!
//! `compiler` automatically enables `runtime` because the upstream C header
//! always exposes runtime symbols alongside the compiler surface.
//!
//! # Example
//!
//! ```no_run
//! use slangmake::{
//!     compiler::Compiler,
//!     runtime::{BlobReader, BlobWriter},
//!     CompileOptions, Permutation, Target,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let compiler = Compiler::new()?;
//! let mut opts = CompileOptions::new()?;
//! opts.input_file("shader.slang")?
//!     .entry_point("main")?
//!     .target(Target::Spirv)
//!     .emit_spirv_directly(true);
//!
//! let perm = Permutation::new()?;
//! let result = compiler.compile(&opts, &perm)?;
//!
//! let mut writer = BlobWriter::new(Target::Spirv)?;
//! writer.add_entry(&perm, result.code(), result.reflection(), &[]);
//! let blob = writer.finalize()?;
//!
//! let reader = BlobReader::open_owned(blob.to_vec())?;
//! assert_eq!(reader.entry_count(), 1);
//! # Ok(()) }
//! ```
//!
//! # Error handling
//!
//! All fallible operations return [`Result`]. The [`Error`] enum maps the
//! upstream `sm_status_t` codes plus the thread-local diagnostic from
//! [`error::last_error_message`]. Compile failures preserve the diagnostics
//! text on [`Error::Compile`].

#![warn(missing_debug_implementations)]

/// Re-export of the raw [`slangmake_sys`] crate for escape-hatch access to
/// FFI types (e.g. the `sm_fmt_*` reflection PODs surfaced by
/// [`runtime::Reflection`]).
pub use slangmake_sys as sys;

mod buffer;
mod enums;
pub mod error;
mod options;
mod permutation;

pub use buffer::Buffer;
pub use enums::{Codec, FloatingPointMode, MatrixLayout, Optimization, Target};
pub use error::{Error, Result};
pub use options::CompileOptions;
pub use permutation::{Permutation, PermutationDefineKind, PermutationDefines};

#[cfg(feature = "runtime")]
pub use permutation::PermutationList;

#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "compiler")]
pub mod compiler;
