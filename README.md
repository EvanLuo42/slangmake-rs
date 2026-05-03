# slangmake-rs

Rust bindings to [slangmake](https://github.com/EvanLuo42/slangmake), a build
tool that compiles Slang shader permutations into a single `.bin` blob holding
every variant's bytecode and full reflection data.

This workspace ships two crates:

| Crate                                     | Purpose                                                                 |
| ----------------------------------------- | ----------------------------------------------------------------------- |
| [`slangmake-sys`](crates/slangmake-sys)   | Raw `bindgen` FFI. Version tracks upstream releases (e.g. `0.4.1`).      |
| [`slangmake`](crates/slangmake)           | Idiomatic safe wrapper. Independent versioning, starts at `0.1.0`.       |

The `slangmake-sys` build script downloads the prebuilt binaries from the
matching upstream GitHub release (verified against the asset's SHA-256 digest
exposed by the GitHub API), runs `bindgen` against the C ABI header, and
stages the runtime DLLs into `target/{profile}/`. Only Windows x86_64 is
shipped today; the per-target table in `build.rs` is one row to extend.

## Features

Both crates expose the same two Cargo features that map onto upstream's
`SLANG_MAKE_EXPOSE_RUNTIME` / `SLANG_MAKE_EXPOSE_COMPILER` preprocessor gates:

| Features enabled         | Linked DLL          | Notes                                              |
| ------------------------ | ------------------- | -------------------------------------------------- |
| `compiler` (± `runtime`) | `slangmake.dll`     | Full library; pulls in slang* and dxcompiler/dxil. |
| `runtime` only           | `slangmake-rt.dll`  | Read side only; no Slang or DXC runtime.           |

In the wrapper crate, `compiler` implies `runtime`.

## Quick start

```rust
use slangmake::{
    compiler::Compiler, runtime::{BlobReader, BlobWriter},
    CompileOptions, Permutation, Target,
};

fn main() {
    let compiler = Compiler::new()?;
    let mut opts = CompileOptions::new()?;
    opts.input_file("shader.slang")?
    .entry_point("main")?
    .target(Target::Spirv)
    .emit_spirv_directly(true);
    
    let perm = Permutation::new()?;
    let result = compiler.compile(&opts, &perm)?;
    
    let mut writer = BlobWriter::new(Target::Spirv)?;
    writer.add_entry(&perm, result.code(), result.reflection(), &[]);
    let blob = writer.finalize()?;
    
    let reader = BlobReader::open_owned(blob.to_vec())?;
    assert_eq!(reader.entry_count(), 1);
}
```

## License

MIT. See [LICENSE](LICENSE).
