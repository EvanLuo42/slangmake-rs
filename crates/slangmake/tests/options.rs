use slangmake::{CompileOptions, FloatingPointMode, MatrixLayout, Optimization, Target};

#[test]
fn options_hash_is_deterministic() {
    let mut a = CompileOptions::new().unwrap();
    let mut b = CompileOptions::new().unwrap();
    a.target(Target::Spirv).optimization(Optimization::High);
    b.target(Target::Spirv).optimization(Optimization::High);
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn options_hash_changes_with_target() {
    let mut a = CompileOptions::new().unwrap();
    let mut b = CompileOptions::new().unwrap();
    a.target(Target::Spirv);
    b.target(Target::Dxil);
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn options_hash_changes_with_define() {
    let mut a = CompileOptions::new().unwrap();
    a.target(Target::Spirv);
    let h0 = a.hash();
    a.add_define("FOO", "1").unwrap();
    let h1 = a.hash();
    assert_ne!(h0, h1);
}

#[test]
fn options_builder_is_chainable() {
    // Just verify the fluent style compiles and runs without panicking.
    let mut opts = CompileOptions::new().unwrap();
    opts.target(Target::Spirv)
        .optimization(Optimization::Default)
        .matrix_layout(MatrixLayout::RowMajor)
        .fp_mode(FloatingPointMode::Default)
        .debug_info(true)
        .warnings_as_errors(false)
        .emit_reflection(true)
        .vulkan_version(13);
    let _ = opts.hash();
}

#[test]
fn options_string_with_nul_returns_error() {
    let mut opts = CompileOptions::new().unwrap();
    let err = opts.add_define("BAD\0NAME", "1").unwrap_err();
    assert!(matches!(err, slangmake::Error::StringNul));
}
