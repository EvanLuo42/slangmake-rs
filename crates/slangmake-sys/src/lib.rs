#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]

#[cfg(not(any(feature = "runtime", feature = "compiler")))]
compile_error!("slangmake-sys requires at least one of the `runtime` or `compiler` features");

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
