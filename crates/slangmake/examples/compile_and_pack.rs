//! End-to-end demo: compile a tiny Slang compute shader to SPIR-V, pack the
//! result into a slangmake blob, and read it back.
//!
//! Run with: `cargo run -p slangmake --example compile_and_pack`

use std::io::Write;

use slangmake::{
    CompileOptions, Permutation, Target,
    compiler::Compiler,
    runtime::{BlobReader, BlobWriter},
};

const SHADER_SOURCE: &str = r#"
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID) {
    // intentionally empty
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Drop the shader source on disk — slangmake compiles via input file.
    let dir = tempfile::tempdir()?;
    let shader_path = dir.path().join("hello.slang");
    let mut f = std::fs::File::create(&shader_path)?;
    f.write_all(SHADER_SOURCE.as_bytes())?;
    drop(f);

    // 2. Configure and run the compiler.
    let compiler = Compiler::new()?;
    let mut options = CompileOptions::new()?;
    options
        .input_file(shader_path.to_str().unwrap())?
        .entry_point("main")?
        .target(Target::Spirv)
        .emit_spirv_directly(true)
        .emit_reflection(true);

    let mut perm = Permutation::new()?;
    perm.add_constant("USE_FOO", "1")?;

    let result = compiler.compile(&options, &perm)?;
    println!(
        "compiled {} bytes of SPIR-V, {} bytes of reflection",
        result.code().len(),
        result.reflection().len()
    );

    // 3. Pack into a blob.
    let mut writer = BlobWriter::new(Target::Spirv)?;
    writer.add_entry(&perm, result.code(), result.reflection(), &[]);
    let blob = writer.finalize()?;
    println!("packed blob is {} bytes", blob.len());

    // 4. Read it back.
    let reader = BlobReader::open_owned(blob.to_vec())?;
    println!(
        "blob contains {} entries, target = {:?}",
        reader.entry_count(),
        reader.target()
    );
    let entry = reader.find(&perm).ok_or("missing entry for permutation")?;
    println!("  key = {}", entry.key());
    println!("  code bytes = {}", entry.code().len());

    Ok(())
}
