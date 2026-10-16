//! The targets Ruby 1.9.2 is built for, as the build scripts of this crate and of the crates
//! over it decide.

#[path = "../targets.rs"]
mod targets;

use targets::{TARGETS, is_built_for};

#[test]
fn the_interpreter_is_built_for_x86_64_and_aarch64_and_nothing_else() {
    assert_eq!(
        TARGETS,
        ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
    );
    assert!(is_built_for("x86_64-unknown-linux-gnu"));
    assert!(is_built_for("aarch64-unknown-linux-gnu"));
    assert!(!is_built_for("riscv64gc-unknown-linux-gnu"));
}

#[test]
fn every_target_has_its_configuration_header() {
    for target in TARGETS {
        let header = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(target)
            .join("ruby/config.h");
        assert!(header.is_file(), "{} is missing", header.display());
    }
}
