use std::ffi::CStr;
use std::ptr::NonNull;
use std::slice;

use slangmake_sys as sys;

use crate::error::{Error, Result, nonnull};
use crate::options::CompileOptions;
use crate::permutation::Permutation;

/// Single-permutation Slang compiler.
///
/// Owns a Slang `IGlobalSession` internally. Construction is moderately
/// expensive (it loads `slang.dll` and friends), so create one and reuse it
/// for every compile.
#[derive(Debug)]
pub struct Compiler {
    handle: NonNull<sys::sm_compiler_t>,
}

impl Compiler {
    pub fn new() -> Result<Self> {
        let h = unsafe { sys::sm_compiler_create() };
        Ok(Self {
            handle: nonnull(h, "sm_compiler_create")?,
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut sys::sm_compiler_t {
        self.handle.as_ptr()
    }

    /// Compile a single permutation. Returns `Err(Error::Compile)` when the
    /// upstream call returns a result whose `success` flag is false; the
    /// diagnostics string is preserved on the error.
    pub fn compile(
        &self,
        options: &CompileOptions,
        permutation: &Permutation,
    ) -> Result<CompileResult> {
        let h = unsafe {
            sys::sm_compiler_compile(self.handle.as_ptr(), options.as_ptr(), permutation.as_ptr())
        };
        let handle = nonnull(h, "sm_compiler_compile")?;
        let success = unsafe { sys::sm_compile_result_success(handle.as_ptr()) } != 0;
        if !success {
            let diagnostics = unsafe {
                let p = sys::sm_compile_result_diagnostics(handle.as_ptr());
                if p.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(p).to_string_lossy().into_owned()
                }
            };
            unsafe { sys::sm_compile_result_destroy(handle.as_ptr()) }
            return Err(Error::Compile { diagnostics });
        }
        Ok(CompileResult { handle })
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        unsafe { sys::sm_compiler_destroy(self.handle.as_ptr()) }
    }
}

/// Successful compile output: bytecode, reflection blob, diagnostics, and
/// the list of source files Slang touched.
#[derive(Debug)]
pub struct CompileResult {
    handle: NonNull<sys::sm_compile_result_t>,
}

impl CompileResult {
    pub fn code(&self) -> &[u8] {
        unsafe {
            let mut size: usize = 0;
            let p = sys::sm_compile_result_code(self.handle.as_ptr(), &mut size);
            if p.is_null() || size == 0 {
                &[]
            } else {
                slice::from_raw_parts(p, size)
            }
        }
    }

    pub fn reflection(&self) -> &[u8] {
        unsafe {
            let mut size: usize = 0;
            let p = sys::sm_compile_result_reflection(self.handle.as_ptr(), &mut size);
            if p.is_null() || size == 0 {
                &[]
            } else {
                slice::from_raw_parts(p, size)
            }
        }
    }

    pub fn diagnostics(&self) -> &str {
        unsafe {
            let p = sys::sm_compile_result_diagnostics(self.handle.as_ptr());
            if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            }
        }
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        let count = unsafe { sys::sm_compile_result_dependency_count(self.handle.as_ptr()) };
        (0..count).map(move |i| unsafe {
            let p = sys::sm_compile_result_dependency(self.handle.as_ptr(), i);
            if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            }
        })
    }
}

impl Drop for CompileResult {
    fn drop(&mut self) {
        unsafe { sys::sm_compile_result_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for Compiler {}
unsafe impl Send for CompileResult {}
