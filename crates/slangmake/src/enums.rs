//! Idiomatic Rust mirrors of the upstream `sm_*_t` enums.
//!
//! Each enum is `repr(i32)` and convertible to/from its `slangmake_sys`
//! counterpart via [`From`]. The numeric discriminants match the C ABI
//! exactly so the conversion is a no-op at the machine level.

use slangmake_sys as sys;

/// Compile target backend selected via [`crate::CompileOptions::target`] and
/// recorded in the blob header by [`crate::runtime::BlobWriter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Target {
    Spirv = sys::sm_target::SM_TARGET_SPIRV as i32,
    Dxil = sys::sm_target::SM_TARGET_DXIL as i32,
    Dxbc = sys::sm_target::SM_TARGET_DXBC as i32,
    Hlsl = sys::sm_target::SM_TARGET_HLSL as i32,
    Glsl = sys::sm_target::SM_TARGET_GLSL as i32,
    Metal = sys::sm_target::SM_TARGET_METAL as i32,
    MetalLib = sys::sm_target::SM_TARGET_METALLIB as i32,
    Wgsl = sys::sm_target::SM_TARGET_WGSL as i32,
}

impl From<Target> for sys::sm_target_t {
    fn from(t: Target) -> Self {
        match t {
            Target::Spirv => sys::sm_target::SM_TARGET_SPIRV,
            Target::Dxil => sys::sm_target::SM_TARGET_DXIL,
            Target::Dxbc => sys::sm_target::SM_TARGET_DXBC,
            Target::Hlsl => sys::sm_target::SM_TARGET_HLSL,
            Target::Glsl => sys::sm_target::SM_TARGET_GLSL,
            Target::Metal => sys::sm_target::SM_TARGET_METAL,
            Target::MetalLib => sys::sm_target::SM_TARGET_METALLIB,
            Target::Wgsl => sys::sm_target::SM_TARGET_WGSL,
        }
    }
}

impl From<sys::sm_target_t> for Target {
    fn from(t: sys::sm_target_t) -> Self {
        match t {
            sys::sm_target::SM_TARGET_SPIRV => Target::Spirv,
            sys::sm_target::SM_TARGET_DXIL => Target::Dxil,
            sys::sm_target::SM_TARGET_DXBC => Target::Dxbc,
            sys::sm_target::SM_TARGET_HLSL => Target::Hlsl,
            sys::sm_target::SM_TARGET_GLSL => Target::Glsl,
            sys::sm_target::SM_TARGET_METAL => Target::Metal,
            sys::sm_target::SM_TARGET_METALLIB => Target::MetalLib,
            sys::sm_target::SM_TARGET_WGSL => Target::Wgsl,
        }
    }
}

/// Optimization level forwarded to the Slang backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Optimization {
    None = sys::sm_optimization::SM_OPT_NONE as i32,
    Default = sys::sm_optimization::SM_OPT_DEFAULT as i32,
    High = sys::sm_optimization::SM_OPT_HIGH as i32,
    Maximal = sys::sm_optimization::SM_OPT_MAXIMAL as i32,
}

impl From<Optimization> for sys::sm_optimization_t {
    fn from(o: Optimization) -> Self {
        match o {
            Optimization::None => sys::sm_optimization::SM_OPT_NONE,
            Optimization::Default => sys::sm_optimization::SM_OPT_DEFAULT,
            Optimization::High => sys::sm_optimization::SM_OPT_HIGH,
            Optimization::Maximal => sys::sm_optimization::SM_OPT_MAXIMAL,
        }
    }
}

/// Compression codec applied to the blob's payload section.
///
/// Note: when reading, [`crate::runtime::BlobReader::compression`] always
/// reports [`Codec::None`] because upstream rewrites the header field after
/// decompressing the payload in place. See that method's docs for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Codec {
    None = sys::sm_codec::SM_CODEC_NONE as i32,
    Lz4 = sys::sm_codec::SM_CODEC_LZ4 as i32,
    Zstd = sys::sm_codec::SM_CODEC_ZSTD as i32,
}

impl From<Codec> for sys::sm_codec_t {
    fn from(c: Codec) -> Self {
        match c {
            Codec::None => sys::sm_codec::SM_CODEC_NONE,
            Codec::Lz4 => sys::sm_codec::SM_CODEC_LZ4,
            Codec::Zstd => sys::sm_codec::SM_CODEC_ZSTD,
        }
    }
}

impl From<sys::sm_codec_t> for Codec {
    fn from(c: sys::sm_codec_t) -> Self {
        match c {
            sys::sm_codec::SM_CODEC_NONE => Codec::None,
            sys::sm_codec::SM_CODEC_LZ4 => Codec::Lz4,
            sys::sm_codec::SM_CODEC_ZSTD => Codec::Zstd,
        }
    }
}

/// Matrix storage layout passed through to the Slang frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MatrixLayout {
    RowMajor = sys::sm_matrix_layout::SM_MATRIX_ROW_MAJOR as i32,
    ColumnMajor = sys::sm_matrix_layout::SM_MATRIX_COLUMN_MAJOR as i32,
}

impl From<MatrixLayout> for sys::sm_matrix_layout_t {
    fn from(m: MatrixLayout) -> Self {
        match m {
            MatrixLayout::RowMajor => sys::sm_matrix_layout::SM_MATRIX_ROW_MAJOR,
            MatrixLayout::ColumnMajor => sys::sm_matrix_layout::SM_MATRIX_COLUMN_MAJOR,
        }
    }
}

/// Floating-point evaluation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FloatingPointMode {
    Default = sys::sm_fp_mode::SM_FP_DEFAULT as i32,
    Fast = sys::sm_fp_mode::SM_FP_FAST as i32,
    Precise = sys::sm_fp_mode::SM_FP_PRECISE as i32,
}

impl From<FloatingPointMode> for sys::sm_fp_mode_t {
    fn from(m: FloatingPointMode) -> Self {
        match m {
            FloatingPointMode::Default => sys::sm_fp_mode::SM_FP_DEFAULT,
            FloatingPointMode::Fast => sys::sm_fp_mode::SM_FP_FAST,
            FloatingPointMode::Precise => sys::sm_fp_mode::SM_FP_PRECISE,
        }
    }
}
