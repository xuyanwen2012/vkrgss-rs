//! Says whether the second interpreter exists for the target.
//!
//! `vkxp-ruby192-sys` is built for one target and is empty for every other. For those this
//! script sets the `cfg` `vkxp_interpreter_absent`, under which the executable only says
//! that it is not built for the target, and its test files are empty.

use std::env;

/// The one target Ruby 1.9.2 is built for, as in the build script of `vkxp-ruby192-sys`.
const TARGET: &str = "x86_64-unknown-linux-gnu";

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let target = env::var("TARGET").expect("cargo sets TARGET");
    if target != TARGET {
        println!("cargo::rustc-cfg=vkxp_interpreter_absent");
    }
}
