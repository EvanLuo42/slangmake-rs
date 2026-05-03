#![cfg(feature = "compiler")]

use std::io::Write;

use slangmake::{
    CompileOptions, Target,
    compiler::{BatchCompiler, Compiler},
    runtime::BlobReader,
};

const SHADER: &str = r#"
// [permutation] MODE={0,1}
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {
#if MODE == 0
    if (dtid.x == 0) { return; }
#else
    if (dtid.x == 1) { return; }
#endif
}
"#;

#[test]
fn batch_compile_file_produces_blob_with_two_entries() {
    let dir = tempfile::tempdir().unwrap();
    let shader_path = dir.path().join("batch.slang");
    let out_path = dir.path().join("batch.slangbin");
    {
        let mut f = std::fs::File::create(&shader_path).unwrap();
        f.write_all(SHADER.as_bytes()).unwrap();
    }

    let compiler = Compiler::new().unwrap();
    let mut batch = BatchCompiler::new(&compiler).unwrap();
    batch.jobs(1).quiet(true).keep_going(false);

    let mut opts = CompileOptions::new().unwrap();
    opts.entry_point("main")
        .unwrap()
        .target(Target::Spirv)
        .emit_spirv_directly(true);

    let output = batch
        .compile_file(
            shader_path.to_str().unwrap(),
            &opts,
            None,
            out_path.to_str().unwrap(),
        )
        .unwrap();

    assert_eq!(
        output.failure_count(),
        0,
        "failures: {:?}",
        output.failures().collect::<Vec<_>>()
    );
    assert!(out_path.exists(), "batch did not write output blob");
    assert!(!output.blob().is_empty());

    // The `// [permutation] MODE={0,1}` magic comment expands into 2 entries.
    let reader = BlobReader::open_file(&out_path).unwrap();
    assert_eq!(reader.entry_count(), 2);
}
