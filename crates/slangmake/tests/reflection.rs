#![cfg(all(feature = "compiler", feature = "runtime"))]

use std::io::Write;

use slangmake::{CompileOptions, Permutation, Target, compiler::Compiler, runtime::Reflection};

const SHADER: &str = r#"
struct Material { float4 albedo; float roughness; };

[[vk::binding(0, 0)]]
ConstantBuffer<Material> gMat;

[[vk::binding(1, 0)]]
RWTexture2D<float4> gOut;

[shader("compute")]
[numthreads(8, 8, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {
    gOut[dtid.xy] = gMat.albedo * gMat.roughness;
}
"#;

fn compile_for_reflection() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("refl.slang");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(SHADER.as_bytes()).unwrap();
    drop(f);

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
    let bytes = result.reflection().to_vec();
    assert!(!bytes.is_empty(), "compile produced empty reflection");
    bytes
}

#[test]
fn reflection_opens_real_blob() {
    let refl_bytes = compile_for_reflection();
    let _ = Reflection::open(&refl_bytes).expect("Reflection::open on real blob");
}

#[test]
fn reflection_rejects_garbage() {
    let err = Reflection::open(&[0u8; 128]).unwrap_err();
    assert!(matches!(
        err,
        slangmake::Error::InvalidReflection | slangmake::Error::NullHandle { .. }
    ));
}

#[test]
fn reflection_has_at_least_one_entry_point() {
    let refl_bytes = compile_for_reflection();
    let r = Reflection::open(&refl_bytes).unwrap();

    let entry_points = r.entry_points();
    assert!(!entry_points.is_empty(), "expected ≥1 entry point");

    let first = &entry_points[0];
    let name = r.string(first.name_str_idx);
    assert_eq!(name, "main");
    // Compute shader thread group should match [numthreads(8, 8, 1)].
    assert_eq!(first.thread_group_size_x, 8);
    assert_eq!(first.thread_group_size_y, 8);
    assert_eq!(first.thread_group_size_z, 1);

    assert_eq!(r.first_entry_point_hash(), {
        ((first.hash_high as u64) << 32) | first.hash_low as u64
    });
}

#[test]
fn reflection_tables_populated_for_real_shader() {
    let refl_bytes = compile_for_reflection();
    let r = Reflection::open(&refl_bytes).unwrap();

    // The shader declares a struct, two resources, and one entry point — every
    // one of these tables must contain at least one record.
    assert!(!r.types().is_empty(), "types table empty");
    assert!(!r.type_layouts().is_empty(), "type_layouts table empty");
    assert!(!r.variables().is_empty(), "variables table empty");
    assert!(!r.var_layouts().is_empty(), "var_layouts table empty");
    assert!(!r.entry_points().is_empty(), "entry_points table empty");
}

#[test]
fn reflection_string_pool_returns_known_names() {
    let refl_bytes = compile_for_reflection();
    let r = Reflection::open(&refl_bytes).unwrap();

    // Every variable name should resolve to a non-empty string via the pool.
    let mut found_main = false;
    for ep in r.entry_points() {
        if r.string(ep.name_str_idx) == "main" {
            found_main = true;
        }
    }
    assert!(found_main, "expected an entry point named `main`");
}

#[test]
fn reflection_string_handles_invalid_index() {
    let refl_bytes = compile_for_reflection();
    let r = Reflection::open(&refl_bytes).unwrap();
    assert_eq!(r.string(slangmake::sys::SM_INVALID_INDEX), "");
    assert_eq!(r.string(u32::MAX - 1), "");
}
