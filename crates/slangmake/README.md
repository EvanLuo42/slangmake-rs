# slangmake

Idiomatic Rust wrapper around [slangmake](https://github.com/EvanLuo42/slangmake), a build tool that compiles Slang shader permutations into a single `.bin` blob holding every variant's bytecode and full reflection data.

Built on [`slangmake-sys`](https://crates.io/crates/slangmake-sys), which does the prebuilt-binary download and bindgen wiring.

## Features

| Feature    | Module                | What it exposes                                          |
| ---------- | --------------------- | -------------------------------------------------------- |
| `runtime`  | `slangmake::runtime`  | `BlobReader`, `BlobWriter`, `Reflection` (read side)     |
| `compiler` | `slangmake::compiler` | `Compiler`, `BatchCompiler` (compile side; implies runtime) |

`compiler` automatically enables `runtime` because the upstream C header always exposes runtime symbols alongside compiler symbols.

## Example

```rust
use slangmake::{
    compiler::Compiler,
    runtime::{BlobReader, BlobWriter},
    CompileOptions, Permutation, Target,
};

let compiler = Compiler::new()?;
let mut opts = CompileOptions::new()?;
opts.input_file("shader.slang")?
    .entry_point("main")?
    .target(Target::Spirv)
    .emit_spirv_directly(true)
    .emit_reflection(true);

let perm = Permutation::new()?;
let result = compiler.compile(&opts, &perm)?;

let mut writer = BlobWriter::new(Target::Spirv)?;
writer.add_entry(&perm, result.code(), result.reflection(), &[]);
let blob = writer.finalize()?;

let reader = BlobReader::open_owned(blob.to_vec())?;
assert_eq!(reader.entry_count(), 1);
let entry = reader.find(&perm).unwrap();
println!("{} bytes of code, {} of reflection", entry.code().len(), entry.reflection().len());
```

See `examples/compile_and_pack.rs` for the full end-to-end flow.

## Error handling

All fallible operations return `slangmake::Result<T>` where the error enum maps the upstream `sm_status_t` codes plus the thread-local diagnostic from `sm_last_error()`. Compile failures preserve the diagnostics text on `Error::Compile`.

## Platform support

Inherits from `slangmake-sys`: Windows x86_64 today, more platforms when upstream publishes them.

## License

MIT.
