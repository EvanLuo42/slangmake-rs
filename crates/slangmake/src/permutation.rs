use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr::NonNull;

use slangmake_sys as sys;

use crate::error::{Error, Result, nonnull};

/// A concrete combination of preprocessor constants and type arguments
/// identifying one shader variant.
///
/// Constants become `#define NAME VALUE` during compilation; type arguments
/// drive Slang's `[permutation type]` axis (each pair specialises a generic
/// type parameter at compile time, no runtime Slang session needed).
///
/// The canonical alphabetised key returned by [`Permutation::key`] is what
/// [`crate::runtime::BlobReader::find`] compares against on lookup.
#[derive(Debug)]
pub struct Permutation {
    handle: NonNull<sys::sm_permutation_t>,
}

impl Permutation {
    pub fn new() -> Result<Self> {
        let h = unsafe { sys::sm_permutation_create() };
        Ok(Self {
            handle: nonnull(h, "sm_permutation_create")?,
        })
    }

    pub(crate) unsafe fn from_raw(handle: NonNull<sys::sm_permutation_t>) -> Self {
        Self { handle }
    }

    pub(crate) fn as_ptr(&self) -> *const sys::sm_permutation_t {
        self.handle.as_ptr()
    }

    pub fn add_constant(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        let name = cstring(name)?;
        let value = cstring(value)?;
        unsafe {
            sys::sm_permutation_add_constant(self.handle.as_ptr(), name.as_ptr(), value.as_ptr())
        }
        Ok(self)
    }

    pub fn add_type_arg(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        let name = cstring(name)?;
        let value = cstring(value)?;
        unsafe {
            sys::sm_permutation_add_type_arg(self.handle.as_ptr(), name.as_ptr(), value.as_ptr())
        }
        Ok(self)
    }

    /// Canonical alphabetical key. The returned slice is invalidated by the
    /// next mutating call on this permutation.
    pub fn key(&self) -> Result<&str> {
        unsafe {
            let p = sys::sm_permutation_key(self.handle.as_ptr());
            if p.is_null() {
                return Ok("");
            }
            Ok(CStr::from_ptr(p).to_str()?)
        }
    }

    pub fn constants(&self) -> impl Iterator<Item = (&str, &str)> {
        let count = unsafe { sys::sm_permutation_constant_count(self.handle.as_ptr()) };
        (0..count).map(move |i| unsafe {
            let n = CStr::from_ptr(sys::sm_permutation_constant_name(self.handle.as_ptr(), i))
                .to_str()
                .unwrap_or_default();
            let v = CStr::from_ptr(sys::sm_permutation_constant_value(self.handle.as_ptr(), i))
                .to_str()
                .unwrap_or_default();
            (n, v)
        })
    }

    pub fn type_args(&self) -> impl Iterator<Item = (&str, &str)> {
        let count = unsafe { sys::sm_permutation_type_arg_count(self.handle.as_ptr()) };
        (0..count).map(move |i| unsafe {
            let n = CStr::from_ptr(sys::sm_permutation_type_arg_name(self.handle.as_ptr(), i))
                .to_str()
                .unwrap_or_default();
            let v = CStr::from_ptr(sys::sm_permutation_type_arg_value(self.handle.as_ptr(), i))
                .to_str()
                .unwrap_or_default();
            (n, v)
        })
    }
}

impl Drop for Permutation {
    fn drop(&mut self) {
        unsafe { sys::sm_permutation_destroy(self.handle.as_ptr()) }
    }
}

/// Whether a permutation axis varies a preprocessor constant or a type
/// parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermutationDefineKind {
    /// `#define NAME=VALUE` style.
    Constant,
    /// Generic type parameter specialisation (e.g. `T = float`).
    Type,
}

impl PermutationDefineKind {
    fn to_sys(self) -> sys::sm_perm_define_kind_t {
        match self {
            PermutationDefineKind::Constant => sys::sm_perm_define_kind::SM_PERM_KIND_CONSTANT,
            PermutationDefineKind::Type => sys::sm_perm_define_kind::SM_PERM_KIND_TYPE,
        }
    }

    fn from_sys(v: sys::sm_perm_define_kind_t) -> Self {
        match v {
            sys::sm_perm_define_kind::SM_PERM_KIND_TYPE => PermutationDefineKind::Type,
            _ => PermutationDefineKind::Constant,
        }
    }
}

/// Declaration of permutation axes — each axis pairs a name with the set of
/// values it can take.
///
/// Build a list manually with [`PermutationDefines::push`], or parse one out
/// of a Slang source file's `// [permutation] NAME={V1,V2,...}` magic
/// comments via [`PermutationDefines::parse_source`] /
/// [`PermutationDefines::parse_file`]. Calling
/// [`PermutationDefines::expand`] returns the Cartesian product as a
/// [`PermutationList`].
#[derive(Debug)]
pub struct PermutationDefines {
    handle: NonNull<sys::sm_perm_define_list_t>,
}

impl PermutationDefines {
    pub fn new() -> Result<Self> {
        let h = unsafe { sys::sm_perm_define_list_create() };
        Ok(Self {
            handle: nonnull(h, "sm_perm_define_list_create")?,
        })
    }

    pub(crate) fn as_ptr(&self) -> *const sys::sm_perm_define_list_t {
        self.handle.as_ptr()
    }

    pub fn push(&mut self, name: &str, kind: PermutationDefineKind, values: &[&str]) -> Result<()> {
        let name = cstring(name)?;
        let owned: Vec<CString> = values.iter().map(|v| cstring(v)).collect::<Result<_>>()?;
        let ptrs: Vec<*const std::os::raw::c_char> = owned.iter().map(|s| s.as_ptr()).collect();
        unsafe {
            sys::sm_perm_define_list_push(
                self.handle.as_ptr(),
                name.as_ptr(),
                kind.to_sys(),
                ptrs.as_ptr(),
                ptrs.len(),
            )
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        unsafe { sys::sm_perm_define_list_size(self.handle.as_ptr()) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, PermutationDefineKind, Vec<&str>)> {
        (0..self.len()).map(move |i| unsafe {
            let name = CStr::from_ptr(sys::sm_perm_define_list_name(self.handle.as_ptr(), i))
                .to_str()
                .unwrap_or_default();
            let kind = PermutationDefineKind::from_sys(sys::sm_perm_define_list_kind(
                self.handle.as_ptr(),
                i,
            ));
            let value_count = sys::sm_perm_define_list_value_count(self.handle.as_ptr(), i);
            let values = (0..value_count)
                .map(|v| {
                    CStr::from_ptr(sys::sm_perm_define_list_value(self.handle.as_ptr(), i, v))
                        .to_str()
                        .unwrap_or_default()
                })
                .collect();
            (name, kind, values)
        })
    }
}

impl Drop for PermutationDefines {
    fn drop(&mut self) {
        unsafe { sys::sm_perm_define_list_destroy(self.handle.as_ptr()) }
    }
}

#[cfg(feature = "runtime")]
impl PermutationDefines {
    pub fn parse_source(src: &str) -> Result<Self> {
        let h = unsafe { sys::sm_parse_permutations_source(src.as_ptr() as *const _, src.len()) };
        Ok(Self {
            handle: nonnull(h, "sm_parse_permutations_source")?,
        })
    }

    pub fn parse_file(path: &Path) -> Result<Self> {
        let path_c = path_cstring(path)?;
        let h = unsafe { sys::sm_parse_permutations_file(path_c.as_ptr()) };
        Ok(Self {
            handle: nonnull(h, "sm_parse_permutations_file")?,
        })
    }

    /// Merge `cli_override` into `file_defines`, returning a new list. The
    /// inputs are unchanged.
    pub fn merge(file_defines: &Self, cli_override: &Self) -> Result<Self> {
        let h = unsafe { sys::sm_merge_permutations(file_defines.as_ptr(), cli_override.as_ptr()) };
        Ok(Self {
            handle: nonnull(h, "sm_merge_permutations")?,
        })
    }

    /// Cartesian-product expansion of all defines into concrete permutations.
    pub fn expand(&self) -> Result<PermutationList> {
        let h = unsafe { sys::sm_expand_permutations(self.as_ptr()) };
        Ok(PermutationList {
            handle: nonnull(h, "sm_expand_permutations")?,
        })
    }
}

/// Cartesian-product expansion of a [`PermutationDefines`] into concrete
/// [`Permutation`] handles.
#[cfg(feature = "runtime")]
#[derive(Debug)]
pub struct PermutationList {
    handle: NonNull<sys::sm_permutation_list_t>,
}

#[cfg(feature = "runtime")]
impl PermutationList {
    pub fn len(&self) -> usize {
        unsafe { sys::sm_permutation_list_size(self.handle.as_ptr()) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Each call clones the underlying permutation; the returned handle is owned.
    pub fn get(&self, index: usize) -> Result<Permutation> {
        let h = unsafe { sys::sm_permutation_list_clone_at(self.handle.as_ptr(), index) };
        Ok(unsafe { Permutation::from_raw(nonnull(h, "sm_permutation_list_clone_at")?) })
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<Permutation>> + '_ {
        (0..self.len()).map(move |i| self.get(i))
    }
}

#[cfg(feature = "runtime")]
impl Drop for PermutationList {
    fn drop(&mut self) {
        unsafe { sys::sm_permutation_list_destroy(self.handle.as_ptr()) }
    }
}

#[cfg(feature = "runtime")]
unsafe impl Send for PermutationList {}

unsafe impl Send for Permutation {}
unsafe impl Send for PermutationDefines {}

pub(crate) fn cstring(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::StringNul)
}

#[cfg(feature = "runtime")]
pub(crate) fn path_cstring(path: &Path) -> Result<CString> {
    let s = path.to_string_lossy().into_owned();
    CString::new(s).map_err(|_| Error::PathNul {
        path: path.to_path_buf(),
    })
}
