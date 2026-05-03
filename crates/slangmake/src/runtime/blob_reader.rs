use std::ffi::CStr;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;
use std::slice;

use slangmake_sys as sys;

use crate::enums::{Codec, Target};
use crate::error::{Error, Result, nonnull};
use crate::permutation::{Permutation, path_cstring};

/// Reader over a slangmake `.bin` blob.
///
/// The lifetime parameter tracks the borrow of the underlying buffer:
/// - `BlobReader::open_borrowed(&buf)` returns `BlobReader<'_>`.
/// - `BlobReader::open_owned(vec)` and `BlobReader::open_file(path)` return
///   `BlobReader<'static>` because the reader owns its bytes.
#[derive(Debug)]
pub struct BlobReader<'data> {
    handle: NonNull<sys::sm_blob_reader_t>,
    _owned: Option<Box<[u8]>>,
    _borrowed: PhantomData<&'data [u8]>,
}

impl<'data> BlobReader<'data> {
    pub fn open_borrowed(data: &'data [u8]) -> Result<Self> {
        let h = unsafe { sys::sm_blob_reader_open_borrowed(data.as_ptr(), data.len()) };
        let handle = nonnull(h, "sm_blob_reader_open_borrowed")?;
        Self::check_valid(handle)?;
        Ok(Self {
            handle,
            _owned: None,
            _borrowed: PhantomData,
        })
    }

    pub fn target(&self) -> Target {
        unsafe { sys::sm_blob_reader_target(self.handle.as_ptr()) }.into()
    }

    pub fn options_hash(&self) -> u64 {
        unsafe { sys::sm_blob_reader_options_hash(self.handle.as_ptr()) }
    }

    /// Reports the compression codec the reader is currently serving.
    ///
    /// **This is not the on-disk codec.** On open, the reader decompresses the
    /// payload in place and rewrites the in-memory `compression` header field
    /// to [`Codec::None`] so that entry offsets stay valid against the
    /// now-uncompressed coordinates. This accessor reads that rewritten field,
    /// so it returns [`Codec::None`] regardless of the codec the producing
    /// [`BlobWriter`] was configured with. The data accessed via
    /// [`BlobEntry::code`] / [`BlobEntry::reflection`] is the original payload.
    ///
    /// [`BlobWriter`]: crate::runtime::BlobWriter
    pub fn compression(&self) -> Codec {
        unsafe { sys::sm_blob_reader_compression(self.handle.as_ptr()) }.into()
    }

    pub fn entry_count(&self) -> usize {
        unsafe { sys::sm_blob_reader_entry_count(self.handle.as_ptr()) }
    }

    pub fn at(&self, index: usize) -> Option<BlobEntry<'_>> {
        let mut out = std::mem::MaybeUninit::<sys::sm_blob_entry_t>::zeroed();
        let s = unsafe { sys::sm_blob_reader_at(self.handle.as_ptr(), index, out.as_mut_ptr()) };
        if s == sys::SM_OK as sys::sm_status_t {
            Some(unsafe { BlobEntry::from_raw(out.assume_init()) })
        } else {
            None
        }
    }

    pub fn find(&self, perm: &Permutation) -> Option<BlobEntry<'_>> {
        let mut out = std::mem::MaybeUninit::<sys::sm_blob_entry_t>::zeroed();
        let s = unsafe {
            sys::sm_blob_reader_find(self.handle.as_ptr(), perm.as_ptr(), out.as_mut_ptr())
        };
        if s == sys::SM_OK as sys::sm_status_t {
            Some(unsafe { BlobEntry::from_raw(out.assume_init()) })
        } else {
            None
        }
    }

    pub fn dependencies(&self) -> impl Iterator<Item = (&str, u64)> {
        let count = unsafe { sys::sm_blob_reader_dependency_count(self.handle.as_ptr()) };
        (0..count).map(move |i| unsafe {
            let p = sys::sm_blob_reader_dependency_path(self.handle.as_ptr(), i);
            let path = if p.is_null() {
                ""
            } else {
                CStr::from_ptr(p).to_str().unwrap_or_default()
            };
            let hash = sys::sm_blob_reader_dependency_hash(self.handle.as_ptr(), i);
            (path, hash)
        })
    }

    fn check_valid(handle: NonNull<sys::sm_blob_reader_t>) -> Result<()> {
        let valid = unsafe { sys::sm_blob_reader_valid(handle.as_ptr()) };
        if valid != 0 {
            Ok(())
        } else {
            unsafe { sys::sm_blob_reader_destroy(handle.as_ptr()) }
            Err(Error::InvalidBlob)
        }
    }
}

impl BlobReader<'static> {
    pub fn open_owned(data: Vec<u8>) -> Result<Self> {
        let boxed = data.into_boxed_slice();
        let h = unsafe { sys::sm_blob_reader_open_copy(boxed.as_ptr(), boxed.len()) };
        let handle = nonnull(h, "sm_blob_reader_open_copy")?;
        Self::check_valid(handle)?;
        // We pass a copy to slangmake so we could drop `boxed` immediately, but
        // keeping it makes the move explicit and matches `open_file`.
        Ok(Self {
            handle,
            _owned: Some(boxed),
            _borrowed: PhantomData,
        })
    }

    pub fn open_file(path: &Path) -> Result<Self> {
        let c = path_cstring(path)?;
        let h = unsafe { sys::sm_blob_reader_open_file(c.as_ptr()) };
        let handle = nonnull(h, "sm_blob_reader_open_file")?;
        Self::check_valid(handle)?;
        Ok(Self {
            handle,
            _owned: None,
            _borrowed: PhantomData,
        })
    }
}

impl Drop for BlobReader<'_> {
    fn drop(&mut self) {
        unsafe { sys::sm_blob_reader_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for BlobReader<'_> {}

/// Borrowed view of one entry inside a [`BlobReader`].
///
/// Slices returned by the accessors point into the reader's internal buffer
/// and live as long as the parent reader.
#[derive(Debug)]
pub struct BlobEntry<'r> {
    raw: sys::sm_blob_entry_t,
    _marker: PhantomData<&'r ()>,
}

impl<'r> BlobEntry<'r> {
    unsafe fn from_raw(raw: sys::sm_blob_entry_t) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn key(&self) -> &'r str {
        unsafe { slice_to_str(self.raw.key, self.raw.key_size) }
    }

    pub fn code(&self) -> &'r [u8] {
        unsafe { slice_or_empty(self.raw.code, self.raw.code_size) }
    }

    pub fn reflection(&self) -> &'r [u8] {
        unsafe { slice_or_empty(self.raw.reflection, self.raw.reflection_size) }
    }

    pub fn dep_indices(&self) -> &'r [u32] {
        unsafe {
            if self.raw.dep_indices.is_null() || self.raw.dep_index_count == 0 {
                &[]
            } else {
                slice::from_raw_parts(self.raw.dep_indices, self.raw.dep_index_count)
            }
        }
    }
}

unsafe fn slice_or_empty<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

unsafe fn slice_to_str<'a>(ptr: *const std::os::raw::c_char, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    let bytes = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
    std::str::from_utf8(bytes).unwrap_or_default()
}
