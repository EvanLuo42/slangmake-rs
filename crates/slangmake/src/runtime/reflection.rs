use std::ffi::CStr;
use std::ptr::NonNull;
use std::slice;

use slangmake_sys as sys;

use crate::error::{Error, Result, nonnull};

/// Zero-copy view over a serialized reflection blob.
///
/// Open one with [`Reflection::open`] from the bytes returned by
/// [`crate::compiler::CompileResult::reflection`] (or
/// [`crate::runtime::BlobEntry::reflection`]).
///
/// The accessors return raw `&[sm_fmt_*_t]` slices straight from the on-disk
/// tables — walking them to compute concrete `(set, slot, byte-offset)`
/// tuples is the consuming engine's RHI concern. See the upstream reflection
/// docs for the table semantics.
#[derive(Debug)]
pub struct Reflection {
    handle: NonNull<sys::sm_reflection_t>,
}

impl Reflection {
    pub fn open(data: &[u8]) -> Result<Self> {
        let h = unsafe { sys::sm_reflection_open(data.as_ptr(), data.len()) };
        let handle = nonnull(h, "sm_reflection_open")?;
        if unsafe { sys::sm_reflection_valid(handle.as_ptr()) } == 0 {
            unsafe { sys::sm_reflection_destroy(handle.as_ptr()) }
            return Err(Error::InvalidReflection);
        }
        Ok(Self { handle })
    }

    pub fn first_entry_point_hash(&self) -> u64 {
        unsafe { sys::sm_reflection_first_entry_point_hash(self.handle.as_ptr()) }
    }

    pub fn global_cb_binding(&self) -> u32 {
        unsafe { sys::sm_reflection_global_cb_binding(self.handle.as_ptr()) }
    }

    pub fn global_cb_size(&self) -> u32 {
        unsafe { sys::sm_reflection_global_cb_size(self.handle.as_ptr()) }
    }

    pub fn bindless_space(&self) -> u32 {
        unsafe { sys::sm_reflection_bindless_space(self.handle.as_ptr()) }
    }

    /// Returns `None` if the blob has no global params layout.
    pub fn global_params_var_layout(&self) -> Option<u32> {
        let v = unsafe { sys::sm_reflection_global_params_var_layout(self.handle.as_ptr()) };
        if v == sys::SM_INVALID_INDEX {
            None
        } else {
            Some(v)
        }
    }

    pub fn string(&self, str_idx: u32) -> &str {
        unsafe {
            let p = sys::sm_reflection_string(self.handle.as_ptr(), str_idx);
            cstr_or_empty(p)
        }
    }

    pub fn hashed_string(&self, index: u32) -> &str {
        unsafe {
            let p = sys::sm_reflection_hashed_string(self.handle.as_ptr(), index);
            cstr_or_empty(p)
        }
    }
}

impl Drop for Reflection {
    fn drop(&mut self) {
        unsafe { sys::sm_reflection_destroy(self.handle.as_ptr()) }
    }
}

unsafe impl Send for Reflection {}

unsafe fn cstr_or_empty<'a>(p: *const std::os::raw::c_char) -> &'a str {
    if p.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(p) }.to_str().unwrap_or_default()
    }
}

macro_rules! reflection_table {
    ($name:ident, $sys_fn:ident, $ty:ty) => {
        impl Reflection {
            pub fn $name(&self) -> &[$ty] {
                let mut count: usize = 0;
                let p = unsafe { sys::$sys_fn(self.handle.as_ptr(), &mut count) };
                if p.is_null() || count == 0 {
                    &[]
                } else {
                    unsafe { slice::from_raw_parts(p, count) }
                }
            }
        }
    };
}

reflection_table!(types, sm_reflection_types, sys::sm_fmt_type_t);
reflection_table!(
    type_layouts,
    sm_reflection_type_layouts,
    sys::sm_fmt_type_layout_t
);
reflection_table!(variables, sm_reflection_variables, sys::sm_fmt_variable_t);
reflection_table!(
    var_layouts,
    sm_reflection_var_layouts,
    sys::sm_fmt_var_layout_t
);
reflection_table!(functions, sm_reflection_functions, sys::sm_fmt_function_t);
reflection_table!(generics, sm_reflection_generics, sys::sm_fmt_generic_t);
reflection_table!(decls, sm_reflection_decls, sys::sm_fmt_decl_t);
reflection_table!(
    entry_points,
    sm_reflection_entry_points,
    sys::sm_fmt_entry_point_t
);
reflection_table!(
    attributes,
    sm_reflection_attributes,
    sys::sm_fmt_attribute_t
);
reflection_table!(
    attribute_args,
    sm_reflection_attribute_args,
    sys::sm_fmt_attr_arg_t
);
reflection_table!(modifier_pool, sm_reflection_modifier_pool, u32);
reflection_table!(
    hashed_strings,
    sm_reflection_hashed_strings,
    sys::sm_fmt_hashed_str_t
);
reflection_table!(
    binding_ranges,
    sm_reflection_binding_ranges,
    sys::sm_fmt_binding_range_t
);
reflection_table!(
    descriptor_sets,
    sm_reflection_descriptor_sets,
    sys::sm_fmt_descriptor_set_t
);
reflection_table!(
    descriptor_ranges,
    sm_reflection_descriptor_ranges,
    sys::sm_fmt_descriptor_range_t
);
reflection_table!(
    sub_object_ranges,
    sm_reflection_sub_object_ranges,
    sys::sm_fmt_sub_object_range_t
);
reflection_table!(u32_pool, sm_reflection_u32_pool, u32);
