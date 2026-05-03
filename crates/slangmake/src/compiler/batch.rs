use std::ffi::CStr;
use std::ptr::NonNull;
use std::slice;

use slangmake_sys as sys;

use crate::enums::Codec;
use crate::error::{Result, nonnull};
use crate::options::CompileOptions;
use crate::permutation::{Permutation, PermutationDefines, cstring};

use super::compiler::Compiler;

/// Bulk compiler: feed it a Slang file plus an optional CLI permutation
/// override, get back a [`BatchOutput`] containing the packed blob and
/// per-permutation diagnostics.
///
/// Borrows its parent [`Compiler`] for its lifetime. Configure parallelism,
/// verbosity, incremental rebuild, and compression with the chained setters.
#[derive(Debug)]
pub struct BatchCompiler<'c> {
    handle: NonNull<sys::sm_batch_compiler_t>,
    _parent: std::marker::PhantomData<&'c Compiler>,
}

impl<'c> BatchCompiler<'c> {
    pub fn new(parent: &'c Compiler) -> Result<Self> {
        let h = unsafe { sys::sm_batch_compiler_create(parent.as_ptr()) };
        Ok(Self {
            handle: nonnull(h, "sm_batch_compiler_create")?,
            _parent: std::marker::PhantomData,
        })
    }

    pub fn keep_going(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_keep_going(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn verbose(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_verbose(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn quiet(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_quiet(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn jobs(&mut self, n: i32) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_jobs(self.handle.as_ptr(), n) }
        self
    }

    pub fn incremental(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_incremental(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn compression(&mut self, codec: Codec) -> &mut Self {
        unsafe { sys::sm_batch_compiler_set_compression(self.handle.as_ptr(), codec.into()) }
        self
    }

    pub fn last_reused(&self) -> usize {
        unsafe { sys::sm_batch_compiler_last_reused(self.handle.as_ptr()) }
    }

    pub fn last_compiled(&self) -> usize {
        unsafe { sys::sm_batch_compiler_last_compiled(self.handle.as_ptr()) }
    }

    /// `cli_override` may be `None`. Always returns a `BatchOutput` on
    /// allocation success even when individual permutations fail; check
    /// `BatchOutput::failures` for per-permutation diagnostics.
    pub fn compile_file(
        &mut self,
        file: &str,
        options: &CompileOptions,
        cli_override: Option<&PermutationDefines>,
        output_path: &str,
    ) -> Result<BatchOutput> {
        let file_c = cstring(file)?;
        let out_c = cstring(output_path)?;
        let cli_ptr = cli_override
            .map(PermutationDefines::as_ptr)
            .unwrap_or(std::ptr::null());
        let h = unsafe {
            sys::sm_batch_compile_file(
                self.handle.as_ptr(),
                file_c.as_ptr(),
                options.as_ptr(),
                cli_ptr,
                out_c.as_ptr(),
            )
        };
        Ok(BatchOutput {
            handle: nonnull(h, "sm_batch_compile_file")?,
        })
    }
}

impl Drop for BatchCompiler<'_> {
    fn drop(&mut self) {
        unsafe { sys::sm_batch_compiler_destroy(self.handle.as_ptr()) }
    }
}

/// Output of one [`BatchCompiler::compile_file`] invocation: the packed
/// blob, the per-permutation list of what was compiled, and the failure
/// diagnostics for any permutations that didn't succeed.
#[derive(Debug)]
pub struct BatchOutput {
    handle: NonNull<sys::sm_batch_output_t>,
}

impl BatchOutput {
    pub fn output_path(&self) -> &str {
        unsafe {
            let p = sys::sm_batch_output_path(self.handle.as_ptr());
            if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            }
        }
    }

    pub fn blob(&self) -> &[u8] {
        unsafe {
            let mut size: usize = 0;
            let p = sys::sm_batch_output_blob(self.handle.as_ptr(), &mut size);
            if p.is_null() || size == 0 {
                &[]
            } else {
                slice::from_raw_parts(p, size)
            }
        }
    }

    pub fn compiled_count(&self) -> usize {
        unsafe { sys::sm_batch_output_compiled_count(self.handle.as_ptr()) }
    }

    pub fn compiled_key(&self, i: usize) -> &str {
        unsafe {
            let p = sys::sm_batch_output_compiled_key(self.handle.as_ptr(), i);
            if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            }
        }
    }

    pub fn compiled_clone(&self, i: usize) -> Result<Permutation> {
        let h = unsafe { sys::sm_batch_output_compiled_clone(self.handle.as_ptr(), i) };
        Ok(unsafe { Permutation::from_raw(nonnull(h, "sm_batch_output_compiled_clone")?) })
    }

    pub fn failures(&self) -> impl Iterator<Item = &str> {
        let count = unsafe { sys::sm_batch_output_failure_count(self.handle.as_ptr()) };
        (0..count).map(move |i| unsafe {
            let p = sys::sm_batch_output_failure(self.handle.as_ptr(), i);
            if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            }
        })
    }

    pub fn failure_count(&self) -> usize {
        unsafe { sys::sm_batch_output_failure_count(self.handle.as_ptr()) }
    }
}

impl Drop for BatchOutput {
    fn drop(&mut self) {
        unsafe { sys::sm_batch_output_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for BatchCompiler<'_> {}
unsafe impl Send for BatchOutput {}
