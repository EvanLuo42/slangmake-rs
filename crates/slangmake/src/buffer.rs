use std::ptr::NonNull;
use std::slice;

use slangmake_sys as sys;

/// Owned byte buffer allocated and freed by slangmake.
///
/// Returned by [`crate::runtime::BlobWriter::finalize`]. The bytes are valid
/// for the lifetime of this `Buffer`; call [`Buffer::to_vec`] when you need
/// the data to outlive the handle.
#[derive(Debug)]
pub struct Buffer {
    handle: NonNull<sys::sm_buffer_t>,
}

impl Buffer {
    pub(crate) unsafe fn from_raw(handle: NonNull<sys::sm_buffer_t>) -> Self {
        Self { handle }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let data = sys::sm_buffer_data(self.handle.as_ptr());
            let size = sys::sm_buffer_size(self.handle.as_ptr());
            if data.is_null() || size == 0 {
                &[]
            } else {
                slice::from_raw_parts(data, size)
            }
        }
    }

    pub fn len(&self) -> usize {
        unsafe { sys::sm_buffer_size(self.handle.as_ptr()) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { sys::sm_buffer_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for Buffer {}
