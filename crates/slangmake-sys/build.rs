use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct PrebuiltTarget {
    /// Suffix on the upstream zip name (`slangmake-vX.Y.Z-{suffix}.zip`).
    asset_suffix: &'static str,
    /// Subdir of the extracted release containing import/static libraries.
    lib_subdir: &'static str,
    /// Subdir containing the runtime shared libraries (DLLs / .so / .dylib).
    runtime_subdir: &'static str,
    /// Cargo `link-lib` name (without prefix/extension) for the full library.
    full_lib: &'static str,
    /// Cargo `link-lib` name for the slim runtime-only library.
    runtime_lib: &'static str,
    /// File extension of the runtime shared library (`dll`, `so`, `dylib`).
    runtime_ext: &'static str,
}

const TARGETS: &[(&str, &str, PrebuiltTarget)] = &[(
    "windows",
    "x86_64",
    PrebuiltTarget {
        asset_suffix: "windows-x86_64",
        lib_subdir: "lib",
        runtime_subdir: "bin",
        full_lib: "slangmake",
        runtime_lib: "slangmake-rt",
        runtime_ext: "dll",
    },
)];

/// GitHub repo that publishes the upstream prebuilt releases.
const RELEASE_REPO: &str = "EvanLuo42/slangmake";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SLANGMAKE_SYS_PREBUILT_DIR");
    println!("cargo:rerun-if-env-changed=GITHUB_TOKEN");

    let feature_runtime = env::var_os("CARGO_FEATURE_RUNTIME").is_some();
    let feature_compiler = env::var_os("CARGO_FEATURE_COMPILER").is_some();
    if !feature_runtime && !feature_compiler {
        // src/lib.rs already raises a compile_error!; nothing useful to do here.
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH");
    let target = pick_target(&target_os, &target_arch).unwrap_or_else(|| {
        panic!(
            "slangmake-sys: no prebuilt is published for target {target_os}-{target_arch}. \
             Open an issue at https://github.com/EvanLuo42/slangmake-rs to request it, \
             or set SLANGMAKE_SYS_PREBUILT_DIR to a locally-built install tree."
        );
    });

    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

    let prebuilt_root = match env::var_os("SLANGMAKE_SYS_PREBUILT_DIR") {
        Some(p) => {
            let root = PathBuf::from(p);
            assert!(
                root.join(target.lib_subdir).is_dir() && root.join("include").is_dir(),
                "SLANGMAKE_SYS_PREBUILT_DIR={} is missing `include/` or `{}/`",
                root.display(),
                target.lib_subdir
            );
            root
        }
        None => fetch_and_extract(&out_dir, &version, target),
    };

    run_bindgen(&prebuilt_root, &out_dir, feature_runtime, feature_compiler);
    emit_link_directives(&prebuilt_root, target, feature_compiler);
    stage_dlls(&prebuilt_root, &out_dir, target, feature_compiler);

    println!("cargo:include={}", prebuilt_root.join("include").display());
    println!("cargo:root={}", prebuilt_root.display());
}

fn pick_target(os: &str, arch: &str) -> Option<PrebuiltTarget> {
    TARGETS
        .iter()
        .find(|(o, a, _)| *o == os && *a == arch)
        .map(|(_, _, t)| *t)
}

fn fetch_and_extract(out_dir: &Path, version: &str, target: PrebuiltTarget) -> PathBuf {
    let cache = out_dir
        .join("prebuilt")
        .join(format!("v{version}-{}", target.asset_suffix));
    let sentinel = cache.join(".extracted");
    if sentinel.is_file() {
        return cache;
    }

    fs::create_dir_all(&cache).expect("create cache dir");
    let zip_path = cache.join("archive.zip");
    let asset_name = format!("slangmake-v{version}-{}.zip", target.asset_suffix);

    let agent = http_agent();
    let asset = lookup_release_asset(&agent, version, &asset_name).unwrap_or_else(|e| {
        panic!(
            "slangmake-sys: could not locate {asset_name} on GitHub release v{version}: {e}\n\
             (set SLANGMAKE_SYS_PREBUILT_DIR to use a local install, or GITHUB_TOKEN to raise the API rate limit)"
        );
    });

    if !zip_path.is_file() {
        // `cargo:warning=` is the only output channel cargo surfaces from a
        // build script by default; we use it here as a one-shot progress note
        // for the ~70 MB download, not because anything is wrong.
        println!(
            "cargo:warning=slangmake-sys: fetching prebuilt v{version} ({asset_name}, ~{} MiB)",
            estimated_size_mib(target.asset_suffix)
        );
        download(&agent, &asset.url, &zip_path).unwrap_or_else(|e| {
            panic!("slangmake-sys: failed to download {}: {e}", asset.url);
        });
    }

    verify_sha256(&zip_path, &asset.sha256, &asset_name);
    extract_zip(&zip_path, &cache).expect("extract release zip");
    fs::write(&sentinel, b"ok").expect("write sentinel");
    cache
}

struct ReleaseAsset {
    url: String,
    sha256: String,
}

fn lookup_release_asset(
    agent: &ureq::Agent,
    version: &str,
    asset_name: &str,
) -> Result<ReleaseAsset, String> {
    let api_url = format!("https://api.github.com/repos/{RELEASE_REPO}/releases/tags/v{version}");
    let mut req = agent
        .get(&api_url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "slangmake-sys-build");
    if let Some(token) = env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()) {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let mut response = req.call().map_err(|e| format!("API request failed: {e}"))?;
    let value: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("API returned invalid JSON: {e}"))?;

    let assets = value
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or_else(|| "release JSON missing `assets` array".to_string())?;

    let asset = assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(asset_name))
        .ok_or_else(|| format!("no asset named `{asset_name}` on this release"))?;

    let url = asset
        .get("browser_download_url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "asset missing `browser_download_url`".to_string())?
        .to_string();
    let digest = asset
        .get("digest")
        .and_then(|d| d.as_str())
        .ok_or_else(|| {
            "asset missing `digest` (likely an old release predating GitHub's digest field)"
                .to_string()
        })?;
    let sha256 = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("unsupported digest algorithm: {digest}"))?
        .to_string();

    Ok(ReleaseAsset { url, sha256 })
}

fn estimated_size_mib(asset_suffix: &str) -> u64 {
    // Rough hint so users know the download is large but bounded. Off by a few
    // MiB across releases is fine — this is just for the progress note.
    match asset_suffix {
        "windows-x86_64" => 70,
        _ => 0,
    }
}

fn http_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(30)))
        .timeout_global(Some(std::time::Duration::from_secs(600)))
        .build();
    ureq::Agent::new_with_config(config)
}

fn download(agent: &ureq::Agent, url: &str, dest: &Path) -> io::Result<()> {
    let response = agent
        .get(url)
        .header("User-Agent", "slangmake-sys-build")
        .call()
        .map_err(|e| io::Error::other(format!("HTTP error: {e}")))?;

    let tmp = dest.with_extension("partial");
    {
        let mut out = fs::File::create(&tmp)?;
        let mut reader = response.into_body().into_reader();
        io::copy(&mut reader, &mut out)?;
        out.flush()?;
    }
    if dest.exists() {
        fs::remove_file(dest)?;
    }
    fs::rename(&tmp, dest)?;
    Ok(())
}

fn verify_sha256(zip_path: &Path, expected: &str, asset_name: &str) {
    let mut hasher = Sha256::new();
    let mut f = fs::File::open(zip_path).expect("open zip for hashing");
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).expect("read zip for hashing");
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let got = hex::encode(hasher.finalize());
    assert_eq!(
        got.to_lowercase(),
        expected.to_lowercase(),
        "slangmake-sys: SHA-256 mismatch for {asset_name} (got {got}, expected {expected} from GitHub API)"
    );
}

fn extract_zip(zip_path: &Path, dest: &Path) -> io::Result<()> {
    let f = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(f)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("open zip: {e}")))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("zip entry: {e}")))?;
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        // Strip the top-level `slangmake-vX.Y.Z-{platform}/` directory.
        let stripped: PathBuf = enclosed.components().skip(1).collect();
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let out_path = dest.join(stripped);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn run_bindgen(
    prebuilt_root: &Path,
    out_dir: &Path,
    feature_runtime: bool,
    feature_compiler: bool,
) {
    let include_dir = prebuilt_root.join("include");
    let header = include_dir.join("slangmake.h");
    let mut builder = bindgen::Builder::default()
        .header(header.to_string_lossy().into_owned())
        .clang_arg(format!("-I{}", include_dir.display()))
        .allowlist_function("sm_.*")
        .allowlist_type("sm_.*")
        .allowlist_var("SM_.*")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if feature_runtime {
        builder = builder.clang_arg("-DSLANG_MAKE_EXPOSE_RUNTIME");
    }
    if feature_compiler {
        builder = builder.clang_arg("-DSLANG_MAKE_EXPOSE_COMPILER");
    }

    let bindings = builder
        .generate()
        .expect("bindgen failed to generate slangmake bindings");
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("write bindings.rs");
}

fn emit_link_directives(prebuilt_root: &Path, target: PrebuiltTarget, feature_compiler: bool) {
    let lib_dir = prebuilt_root.join(target.lib_subdir);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    let lib = if feature_compiler {
        target.full_lib
    } else {
        target.runtime_lib
    };
    println!("cargo:rustc-link-lib=dylib={lib}");
}

fn stage_dlls(
    prebuilt_root: &Path,
    out_dir: &Path,
    target: PrebuiltTarget,
    feature_compiler: bool,
) {
    let bin_dir = prebuilt_root.join(target.runtime_subdir);

    // Runtime-only mode pulls just the slim DLL; the full library mode needs
    // every DLL upstream ships in `bin/` (slangmake + slang + dxc + ...).
    let dlls: Vec<PathBuf> = if feature_compiler {
        match collect_runtime_libs(&bin_dir, target.runtime_ext) {
            Ok(v) => v,
            Err(e) => {
                println!(
                    "cargo:warning=slangmake-sys: could not enumerate {}: {e}",
                    bin_dir.display()
                );
                return;
            }
        }
    } else {
        vec![bin_dir.join(format!("{}.{}", target.runtime_lib, target.runtime_ext))]
    };

    let Some(profile_dir) = locate_profile_dir(out_dir) else {
        println!(
            "cargo:warning=slangmake-sys: could not locate target/{{profile}}/; runtime libraries were not staged. \
             Add {} to your library search path manually.",
            bin_dir.display()
        );
        return;
    };
    let deps_dir = profile_dir.join("deps");

    for src in &dlls {
        if !src.is_file() {
            println!(
                "cargo:warning=slangmake-sys: expected runtime library {} not found",
                src.display()
            );
            continue;
        }
        let Some(name) = src.file_name() else {
            continue;
        };
        for dst_dir in [&profile_dir, &deps_dir] {
            let _ = fs::create_dir_all(dst_dir);
            let dst = dst_dir.join(name);
            if let Err(e) = copy_if_changed(src, &dst) {
                println!(
                    "cargo:warning=slangmake-sys: failed to copy {} -> {}: {e}",
                    src.display(),
                    dst.display()
                );
            }
        }
    }
}

fn collect_runtime_libs(dir: &Path, ext: &str) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
    Ok(out)
}

fn copy_if_changed(src: &Path, dst: &Path) -> io::Result<()> {
    let src_meta = fs::metadata(src)?;
    if let Ok(dst_meta) = fs::metadata(dst) {
        if dst_meta.len() == src_meta.len() {
            if let (Ok(s), Ok(d)) = (src_meta.modified(), dst_meta.modified()) {
                if d >= s {
                    return Ok(());
                }
            }
        }
    }
    fs::copy(src, dst)?;
    Ok(())
}

/// `OUT_DIR` is `.../target/{profile}/build/{crate}-{hash}/out`. Walk up the
/// ancestor chain until we find a `build/` directory and return its parent
/// (i.e. `.../target/{profile}/`).
fn locate_profile_dir(out_dir: &Path) -> Option<PathBuf> {
    for ancestor in out_dir.ancestors() {
        if ancestor.file_name().and_then(|n| n.to_str()) == Some("build") {
            return ancestor.parent().map(Path::to_path_buf);
        }
    }
    None
}
