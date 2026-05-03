//! Error type and helpers for mapping `sm_status_t` codes into [`Result`].
//!
//! Every fallible call in the wrapper returns [`Result<T>`]. The error variant
//! captures both the `sm_status_t` code returned by the upstream call and the
//! thread-local diagnostic string set by `sm_last_error()` (when present).

use std::ffi::CStr;
use std::ptr::NonNull;

use slangmake_sys as sys;

/// Error returned by every fallible operation in the wrapper.
///
/// Variants mirror the `SM_ERR_*` status codes published by the C ABI plus a
/// few wrapper-only cases (interior NUL bytes, UTF-8 decoding, malformed
/// blobs). Every IPC-style variant carries a static `context` label naming
/// the upstream entrypoint that failed and a free-form `message` lifted from
/// `sm_last_error()`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid handle ({context}): {message}")]
    InvalidHandle {
        context: &'static str,
        message: String,
    },
    #[error("invalid argument ({context}): {message}")]
    InvalidArg {
        context: &'static str,
        message: String,
    },
    #[error("not found ({context}): {message}")]
    NotFound {
        context: &'static str,
        message: String,
    },
    #[error("I/O error ({context}): {message}")]
    Io {
        context: &'static str,
        message: String,
    },
    #[error("internal error ({context}): {message}")]
    Internal {
        context: &'static str,
        message: String,
    },
    #[error("compile failed: {diagnostics}")]
    Compile { diagnostics: String },
    #[error("blob is invalid (failed validation)")]
    InvalidBlob,
    #[error("reflection blob is invalid")]
    InvalidReflection,
    #[error("slangmake returned a null handle ({context}): {message}")]
    NullHandle {
        context: &'static str,
        message: String,
    },
    #[error("string returned by slangmake was not valid UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("path {path:?} contained an interior NUL byte")]
    PathNul { path: std::path::PathBuf },
    #[error("string contained an interior NUL byte")]
    StringNul,
}

/// Convenience alias: every fallible wrapper API returns `Result<T>` with
/// [`Error`] as the default error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Reads the thread-local diagnostic set by the most recent slangmake call
/// on this thread. Returns an empty string when no error has been recorded.
pub fn last_error_message() -> String {
    unsafe {
        let p = sys::sm_last_error();
        if p.is_null() {
            return String::new();
        }
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

pub(crate) fn check_status(status: sys::sm_status_t, context: &'static str) -> Result<()> {
    let msg = || last_error_message();
    if status == sys::SM_OK as sys::sm_status_t {
        Ok(())
    } else if status == sys::SM_ERR_INVALID_HANDLE {
        Err(Error::InvalidHandle {
            context,
            message: msg(),
        })
    } else if status == sys::SM_ERR_INVALID_ARG {
        Err(Error::InvalidArg {
            context,
            message: msg(),
        })
    } else if status == sys::SM_ERR_NOT_FOUND {
        Err(Error::NotFound {
            context,
            message: msg(),
        })
    } else if status == sys::SM_ERR_IO {
        Err(Error::Io {
            context,
            message: msg(),
        })
    } else {
        Err(Error::Internal {
            context,
            message: msg(),
        })
    }
}

pub(crate) fn nonnull<T>(p: *mut T, context: &'static str) -> Result<NonNull<T>> {
    NonNull::new(p).ok_or_else(|| Error::NullHandle {
        context,
        message: last_error_message(),
    })
}
