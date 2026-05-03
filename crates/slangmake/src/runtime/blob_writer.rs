use std::path::Path;
use std::ptr::NonNull;

use slangmake_sys as sys;

use crate::buffer::Buffer;
use crate::enums::{Codec, Target};
use crate::error::{Result, check_status, nonnull};
use crate::permutation::{Permutation, cstring, path_cstring};

/// One dependency record stored in the blob's dependency table.
///
/// `content_hash` is the FNV-1a 64 hash of the file's contents at compile
/// time, used by incremental rebuild to detect upstream changes.
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub path: String,
    pub content_hash: u64,
}

/// Builder for a slangmake `.bin` blob.
///
/// Add per-permutation entries with [`BlobWriter::add_entry`], optionally
/// record dependencies and an options hash, then [`BlobWriter::finalize`] to
/// produce a [`Buffer`] (or [`BlobWriter::write_to_file`] to stream straight
/// to disk).
#[derive(Debug)]
pub struct BlobWriter {
    handle: NonNull<sys::sm_blob_writer_t>,
}

impl BlobWriter {
    pub fn new(target: Target) -> Result<Self> {
        let h = unsafe { sys::sm_blob_writer_create(target.into()) };
        Ok(Self {
            handle: nonnull(h, "sm_blob_writer_create")?,
        })
    }

    pub fn add_entry(
        &mut self,
        permutation: &Permutation,
        code: &[u8],
        reflection: &[u8],
        dep_indices: &[u32],
    ) {
        let dep_ptr = if dep_indices.is_empty() {
            std::ptr::null()
        } else {
            dep_indices.as_ptr()
        };
        unsafe {
            sys::sm_blob_writer_add_entry(
                self.handle.as_ptr(),
                permutation.as_ptr(),
                code.as_ptr(),
                code.len(),
                reflection.as_ptr(),
                reflection.len(),
                dep_ptr,
                dep_indices.len(),
            )
        }
    }

    pub fn set_dependencies(&mut self, deps: &[DependencyInfo]) -> Result<()> {
        // Keep the CStrings alive for the duration of the call.
        let owned: Vec<_> = deps
            .iter()
            .map(|d| cstring(&d.path).map(|c| (c, d.content_hash)))
            .collect::<Result<_>>()?;
        let raw: Vec<sys::sm_dep_info_t> = owned
            .iter()
            .map(|(c, hash)| sys::sm_dep_info_t {
                path: c.as_ptr(),
                content_hash: *hash,
            })
            .collect();
        unsafe {
            sys::sm_blob_writer_set_dependencies(self.handle.as_ptr(), raw.as_ptr(), raw.len())
        }
        Ok(())
    }

    pub fn set_options_hash(&mut self, hash: u64) {
        unsafe { sys::sm_blob_writer_set_options_hash(self.handle.as_ptr(), hash) }
    }

    pub fn set_compression(&mut self, codec: Codec) {
        unsafe { sys::sm_blob_writer_set_compression(self.handle.as_ptr(), codec.into()) }
    }

    pub fn entry_count(&self) -> usize {
        unsafe { sys::sm_blob_writer_entry_count(self.handle.as_ptr()) }
    }

    pub fn finalize(&self) -> Result<Buffer> {
        let h = unsafe { sys::sm_blob_writer_finalize(self.handle.as_ptr()) };
        Ok(unsafe { Buffer::from_raw(nonnull(h, "sm_blob_writer_finalize")?) })
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let c = path_cstring(path)?;
        let s = unsafe { sys::sm_blob_writer_write_to_file(self.handle.as_ptr(), c.as_ptr()) };
        check_status(s, "sm_blob_writer_write_to_file")
    }
}

impl Drop for BlobWriter {
    fn drop(&mut self) {
        unsafe { sys::sm_blob_writer_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for BlobWriter {}
