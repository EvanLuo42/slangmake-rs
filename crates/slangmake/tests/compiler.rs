#![cfg(feature = "compiler")]

use std::io::Write;

use slangmake::{CompileOptions, Error, Permutation, Target, compiler::Compiler};

const VALID_SHADER: &str = r#"
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {}
"#;

const BROKEN_SHADER: &str = r#"
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {
    this_function_does_not_exist();
}
"#;

fn write_shader(src: &str, name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(src.as_bytes()).unwrap();
    (dir, path)
}

#[test]
fn compiler_new_succeeds() {
    // Just creating a Compiler exercises slangmake.dll + slang*.dll loading;
    // if the build script didn't stage the runtime DLLs this would fail.
    let _ = Compiler::new().unwrap();
}

#[test]
fn compile_minimal_shader_produces_spirv() {
    let (_dir, path) = write_shader(VALID_SHADER, "ok.slang");

    let compiler = Compiler::new().unwrap();
    let mut opts = CompileOptions::new().unwrap();
    opts.input_file(path.to_str().unwrap())
        .unwrap()
        .entry_point("main")
        .unwrap()
        .target(Target::Spirv)
        .emit_spirv_directly(true)
        .emit_reflection(true);
    let perm = Permutation::new().unwrap();

    let result = compiler.compile(&opts, &perm).unwrap();
    assert!(!result.code().is_empty(), "expected non-empty SPIR-V");
    assert!(!result.reflection().is_empty(), "expected reflection");

    // SPIR-V magic word is 0x07230203 (little-endian on disk).
    let head = &result.code()[..4];
    assert_eq!(head, &[0x03, 0x02, 0x23, 0x07]);

    // Dependencies should at least include the input file.
    let deps: Vec<&str> = result.dependencies().collect();
    assert!(
        deps.iter().any(|d| d.contains("ok.slang")),
        "input file missing from dependencies: {deps:?}"
    );
}

#[test]
fn compile_broken_shader_returns_compile_error() {
    let (_dir, path) = write_shader(BROKEN_SHADER, "bad.slang");

    let compiler = Compiler::new().unwrap();
    let mut opts = CompileOptions::new().unwrap();
    opts.input_file(path.to_str().unwrap())
        .unwrap()
        .entry_point("main")
        .unwrap()
        .target(Target::Spirv)
        .emit_spirv_directly(true);
    let perm = Permutation::new().unwrap();

    let err = compiler.compile(&opts, &perm).unwrap_err();
    match err {
        Error::Compile { diagnostics } => {
            assert!(
                !diagnostics.is_empty(),
                "expected upstream diagnostics in error"
            );
        }
        other => panic!("expected Error::Compile, got {other:?}"),
    }
}

#[test]
fn permutation_define_changes_compiled_output() {
    // Same source, different permutation define — the bound resource is what
    // forces a real bytecode difference (a no-op `if (...) { return; }` is
    // dead-code-eliminated and would not surface the permutation in SPIR-V).
    const PERM_SHADER: &str = r#"
RWStructuredBuffer<uint> gOut;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {
#if MODE == 0
    gOut[dtid.x] = 42;
#else
    gOut[dtid.x] = 1337;
#endif
}
"#;
    let (_dir, path) = write_shader(PERM_SHADER, "perm.slang");

    let compiler = Compiler::new().unwrap();
    let mut opts = CompileOptions::new().unwrap();
    opts.input_file(path.to_str().unwrap())
        .unwrap()
        .entry_point("main")
        .unwrap()
        .target(Target::Spirv)
        .emit_spirv_directly(true);

    let mut perm0 = Permutation::new().unwrap();
    perm0.add_constant("MODE", "0").unwrap();
    let mut perm1 = Permutation::new().unwrap();
    perm1.add_constant("MODE", "1").unwrap();

    let r0 = compiler.compile(&opts, &perm0).unwrap();
    let r1 = compiler.compile(&opts, &perm1).unwrap();
    assert_ne!(
        r0.code(),
        r1.code(),
        "permutation define did not affect SPIR-V"
    );
}
