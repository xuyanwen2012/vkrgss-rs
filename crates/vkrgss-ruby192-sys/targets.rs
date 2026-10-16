//! The targets Ruby 1.9.2 is built for, in one place. The build script of this crate and
//! those of `vkrgss-binding-rgss3`, `vkrgss-guest-main-rgss3` and `vkrgss-guest-rgss3` include
//! this file, so that they decide alike; `tests/targets.rs` tests it. The `cfg` of the
//! dependencies in `Cargo.toml` names the same targets.

/// The targets Ruby 1.9.2 is built for, each with its configuration header under `config/`
/// in the directory of its name. For any other the crate is empty.
pub(crate) const TARGETS: &[&str] = &["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"];

/// Whether Ruby 1.9.2 is built for `target`.
pub(crate) fn is_built_for(target: &str) -> bool {
    TARGETS.contains(&target)
}
