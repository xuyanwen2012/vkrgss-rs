# Design

## Context

- `crates/vkxp-host/src/lib.rs` declares `pub mod demo;` and `pub mod testing;`.
- Users: the crate's unit tests (`session/tests.rs`, `windowed.rs`), its integration tests
  (`tests/demo.rs`, `tests/headless.rs`, `tests/common/mod.rs`), and its examples (`demo`,
  `demo-headless`). `vkxp` depends on `vkxp-host` and uses neither module.
- The gate runs `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets`
  without naming features.

## Decisions

- A feature `testing` in `vkxp-host`, off by default. Both modules are declared under
  `#[cfg(any(test, feature = "testing"))]`.
- The crate lists itself in `[dev-dependencies]` with the feature
  (`vkxp-host = { path = ".", features = ["testing"] }`), so its integration tests and
  examples see the modules under plain `cargo test` and `cargo run --example` with no flag,
  and the gate's commands stay as they are. If cargo refuses that for a workspace dependency,
  the examples and integration tests get `required-features = ["testing"]` and the gate's
  `clippy` and `test` steps pass `--features vkxp-host/testing`; say which was used.
- Items of the production modules that exist only for the doubles (for example a constructor
  made public for them) get the same `cfg`, where that is a one-line change.
- A check that the executable is built without the feature: `cargo tree -p vkxp -e features`
  does not list `vkxp-host feature "testing"`, as a test in `xtask` next to the dependency
  rules or as a documented command; the test is preferred if it is a few lines.

## Risks / Trade-offs

- [Feature unification turns the feature on for the executable in a workspace build] → With
  resolver 2 a dev-dependency's features do not reach a normal build of another package; the
  check above shows it.

## Follow-ups

- Whether the demo guest is still worth keeping now that games run.
