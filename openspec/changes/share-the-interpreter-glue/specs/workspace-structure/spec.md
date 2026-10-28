## MODIFIED Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkrgss-proto`,
`vkrgss-pixel`, `vkrgss-core`, `vkrgss-image`, `vkrgss-text`, `vkrgss-vfs`, `vkrgss-ruby-glue`,
`vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-binding`, `vkrgss-binding-rgss3`,
`vkrgss-guest`, `vkrgss-guest-main`, `vkrgss-guest-main-rgss3`, `vkrgss-compose-ref`,
`vkrgss-vk`, `vkrgss-audio`, `vkrgss-host`, `vkrgss-android`, `xtask` and the binaries
`vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings, and the directory of the build holds the executables `vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3`

#### Scenario: The crates of the second interpreter on another target
- **WHEN** the workspace is built for a target other than `x86_64-unknown-linux-gnu`
- **THEN** `vkrgss-ruby192-sys`, `vkrgss-binding-rgss3` and `vkrgss-guest-main-rgss3` build and are empty, `vkrgss-guest-rgss3` builds an executable that only says it is not built for the target, and every other crate builds as before

## ADDED Requirements

### Requirement: Interpreter glue in one place
The Rust code that bridges host functions, rooting and protected calls to an interpreter SHALL
exist once, in `vkrgss-ruby-glue`, and both interpreter crates SHALL use it. The glue crate
SHALL contain no C code and export no unmangled symbol.

#### Scenario: No copy
- **WHEN** the sources of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` are read
- **THEN** neither has its own `root.rs` or `host.rs`

#### Scenario: Both interpreters
- **WHEN** the tests of both interpreter crates and both binding packages run
- **THEN** they pass as before
