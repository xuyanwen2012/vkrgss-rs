//! The targets Ruby 1.9.2 is built for, as the build scripts of this crate and of the crates
//! over it decide.

#[path = "../targets.rs"]
mod targets;

use targets::{TARGETS, is_built_for};

#[test]
fn the_interpreter_is_built_for_x86_64_aarch64_and_android_and_nothing_else() {
    assert_eq!(
        TARGETS,
        [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "aarch64-linux-android"
        ]
    );
    assert!(is_built_for("x86_64-unknown-linux-gnu"));
    assert!(is_built_for("aarch64-unknown-linux-gnu"));
    assert!(is_built_for("aarch64-apple-darwin"));
    assert!(is_built_for("aarch64-linux-android"));
    assert!(!is_built_for("riscv64gc-unknown-linux-gnu"));
    assert!(!is_built_for("x86_64-linux-android"));
}

#[test]
fn every_target_has_its_configuration_header() {
    for target in TARGETS {
        let header = std::env::var_os("CARGO_MANIFEST_DIR")
            .map_or_else(
                || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                std::path::PathBuf::from,
            )
            .join("config")
            .join(target)
            .join("ruby/config.h");
        assert!(header.is_file(), "{} is missing", header.display());
    }
}
