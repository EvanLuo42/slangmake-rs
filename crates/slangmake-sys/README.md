# slangmake-sys

Raw `bindgen` FFI bindings for [slangmake](https://github.com/EvanLuo42/slangmake).

This crate's version tracks the upstream slangmake release exactly: `slangmake-sys = "0.4.1"` consumes the `v0.4.1` GitHub release.

For an idiomatic Rust API see the [`slangmake`](https://crates.io/crates/slangmake) wrapper crate.

## How the build works

The `build.rs` queries the GitHub API for the release tagged `v{CARGO_PKG_VERSION}`, downloads the platform-matching prebuilt zip, verifies it against the SHA-256 digest the GitHub API publishes, extracts headers + import libs + DLLs, runs `bindgen` against `slangmake.h`, and stages the runtime DLLs into `target/{profile}/` so `cargo run` / `cargo test` can find them.

### Environment overrides

| Variable                       | Effect                                                                            |
| ------------------------------ | --------------------------------------------------------------------------------- |
| `SLANGMAKE_SYS_PREBUILT_DIR`   | Point at an already-extracted release tree (`include/`, `lib/`, `bin/`); skips download. Use for vendored builds, CI mirrors, or offline development. |
| `GITHUB_TOKEN`                 | Sent as a bearer token on the GitHub API request so CI doesn't hit the 60/hour anonymous rate limit. |

### Features

| Feature    | Selects                              | Defines passed to bindgen          |
| ---------- | ------------------------------------ | ---------------------------------- |
| `runtime`  | links `slangmake-rt.dll`             | `SLANG_MAKE_EXPOSE_RUNTIME`        |
| `compiler` | links `slangmake.dll` (full library) | `SLANG_MAKE_EXPOSE_COMPILER` (+ runtime) |

At least one feature must be enabled. `compiler` is the superset and links the full DLL plus its slang/dxc runtime dependencies; `runtime`-only is for consumers that just need to crack open shipped blobs without redistributing the Slang stack.

## Platform support

| OS        | Arch    | Status     |
| --------- | ------- | ---------- |
| Windows   | x86_64  | shipped    |
| Linux     | x86_64  | TODO (waiting on upstream prebuilt) |
| macOS     | aarch64 | TODO       |

Adding a target is one entry in the `TARGETS` table in `build.rs` once upstream publishes the corresponding zip.

## License

MIT.
