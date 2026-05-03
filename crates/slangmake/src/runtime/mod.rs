//! Read side of slangmake: open packed blobs, walk reflection tables, and
//! synthesize new blobs from compiled bytecode.
//!
//! This module is gated by the `runtime` feature and is also enabled
//! transitively whenever `compiler` is on (the compiler returns code +
//! reflection slices that you typically pack with [`BlobWriter`]).

mod blob_reader;
mod blob_writer;
mod reflection;

pub use blob_reader::{BlobEntry, BlobReader};
pub use blob_writer::{BlobWriter, DependencyInfo};
pub use reflection::Reflection;
