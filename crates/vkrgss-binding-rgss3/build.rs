//! Says whether the second interpreter exists for the target, and that the binding is
//! compiled over it.
//!
//! `vkrgss-ruby192-sys` is built for the targets of its `targets.rs` and is empty for every
//! other. For those this script sets the `cfg` `vkrgss_interpreter_absent`, which empties the
//! library of this package and its test files.

/// The targets Ruby 1.9.2 is built for, the file that the build script of
/// `vkrgss-ruby192-sys` includes.
#[path = "../vkrgss-ruby192-sys/targets.rs"]
mod ruby192_targets;

use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=../vkrgss-ruby192-sys/targets.rs");
    let target = env::var("TARGET").expect("cargo sets TARGET");
    if !ruby192_targets::is_built_for(&target) {
        println!("cargo::rustc-cfg=vkrgss_interpreter_absent");
    }
    // The source of the binding reaches the functions of Ruby 1.9.2 that Ruby 1.8.7 has no
    // counterpart of, such as those of encodings, under this `cfg` only.
    println!("cargo::rustc-cfg=vkrgss_ruby_19");
}
