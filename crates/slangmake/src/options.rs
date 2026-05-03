use std::ptr::NonNull;

use slangmake_sys as sys;

use crate::enums::{FloatingPointMode, MatrixLayout, Optimization, Target};
use crate::error::{Result, nonnull};
use crate::permutation::cstring;

/// Builder for the per-compile knobs: input file, entry point, target,
/// profile, includes, defines, optimisation level, layout/FP modes, etc.
///
/// All setters return `&mut Self` so calls can chain. String-taking setters
/// return [`Result`] because the underlying C API requires NUL-free input.
#[derive(Debug)]
pub struct CompileOptions {
    handle: NonNull<sys::sm_compile_options_t>,
}

impl CompileOptions {
    pub fn new() -> Result<Self> {
        let h = unsafe { sys::sm_compile_options_create() };
        Ok(Self {
            handle: nonnull(h, "sm_compile_options_create")?,
        })
    }

    #[cfg(feature = "compiler")]
    pub(crate) fn as_ptr(&self) -> *const sys::sm_compile_options_t {
        self.handle.as_ptr()
    }

    pub fn input_file(&mut self, path: &str) -> Result<&mut Self> {
        let c = cstring(path)?;
        unsafe { sys::sm_compile_options_set_input_file(self.handle.as_ptr(), c.as_ptr()) }
        Ok(self)
    }

    pub fn entry_point(&mut self, name: &str) -> Result<&mut Self> {
        let c = cstring(name)?;
        unsafe { sys::sm_compile_options_set_entry_point(self.handle.as_ptr(), c.as_ptr()) }
        Ok(self)
    }

    pub fn target(&mut self, t: Target) -> &mut Self {
        unsafe { sys::sm_compile_options_set_target(self.handle.as_ptr(), t.into()) }
        self
    }

    pub fn profile(&mut self, profile: &str) -> Result<&mut Self> {
        let c = cstring(profile)?;
        unsafe { sys::sm_compile_options_set_profile(self.handle.as_ptr(), c.as_ptr()) }
        Ok(self)
    }

    pub fn add_include_path(&mut self, path: &str) -> Result<&mut Self> {
        let c = cstring(path)?;
        unsafe { sys::sm_compile_options_add_include_path(self.handle.as_ptr(), c.as_ptr()) }
        Ok(self)
    }

    pub fn add_define(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        let n = cstring(name)?;
        let v = cstring(value)?;
        unsafe { sys::sm_compile_options_add_define(self.handle.as_ptr(), n.as_ptr(), v.as_ptr()) }
        Ok(self)
    }

    pub fn clear_defines(&mut self) -> &mut Self {
        unsafe { sys::sm_compile_options_clear_defines(self.handle.as_ptr()) }
        self
    }

    pub fn optimization(&mut self, o: Optimization) -> &mut Self {
        unsafe { sys::sm_compile_options_set_optimization(self.handle.as_ptr(), o.into()) }
        self
    }

    pub fn matrix_layout(&mut self, l: MatrixLayout) -> &mut Self {
        unsafe { sys::sm_compile_options_set_matrix_layout(self.handle.as_ptr(), l.into()) }
        self
    }

    pub fn fp_mode(&mut self, m: FloatingPointMode) -> &mut Self {
        unsafe { sys::sm_compile_options_set_fp_mode(self.handle.as_ptr(), m.into()) }
        self
    }

    pub fn debug_info(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_debug_info(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn warnings_as_errors(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_warnings_as_errors(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn glsl_scalar_layout(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_glsl_scalar_layout(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn emit_spirv_directly(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_emit_spirv_directly(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn dump_intermediates(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_dump_intermediates(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn emit_reflection(&mut self, on: bool) -> &mut Self {
        unsafe { sys::sm_compile_options_set_emit_reflection(self.handle.as_ptr(), on as i32) }
        self
    }

    pub fn vulkan_version(&mut self, v: i32) -> &mut Self {
        unsafe { sys::sm_compile_options_set_vulkan_version(self.handle.as_ptr(), v) }
        self
    }

    pub fn add_vulkan_bind_shift(&mut self, kind: u32, space: u32, shift: u32) -> &mut Self {
        unsafe {
            sys::sm_compile_options_add_vulkan_bind_shift(self.handle.as_ptr(), kind, space, shift)
        }
        self
    }

    pub fn language_version(&mut self, v: &str) -> Result<&mut Self> {
        let c = cstring(v)?;
        unsafe { sys::sm_compile_options_set_language_version(self.handle.as_ptr(), c.as_ptr()) }
        Ok(self)
    }

    /// FNV-1a 64 fingerprint of every option field that affects bytecode identity.
    pub fn hash(&self) -> u64 {
        unsafe { sys::sm_compile_options_hash(self.handle.as_ptr()) }
    }
}

impl Drop for CompileOptions {
    fn drop(&mut self) {
        unsafe { sys::sm_compile_options_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for CompileOptions {}
