# workspace-structure Specification

## Purpose
Defines the Cargo workspace layout, the dependency-direction and purity rules between crates, and the single merge gate that enforces them.

## Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkxp-proto`,
`vkxp-pixel`, `vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`, `vkxp-compose-ref`, `vkxp-vk`, `vkxp-audio`, `vkxp-host` and the binary `vkxp`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`) SHALL depend, directly or transitively, on a host crate (`vkxp-vk`,
`vkxp-audio`, `vkxp-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

### Requirement: Core stays pure
`vkxp-core` SHALL contain no `unsafe` code and SHALL NOT depend on Ruby, Vulkan, windowing,
audio or file-system crates.

#### Scenario: Unsafe code in core is rejected
- **WHEN** `unsafe` is introduced in `vkxp-core`
- **THEN** compilation fails

#### Scenario: Impure dependency in core is rejected
- **WHEN** `vkxp-core` gains a dependency outside its allow-list
- **THEN** the merge gate fails and names the dependency

### Requirement: Single merge gate
The repository SHALL provide one command that checks formatting, lints with warnings denied,
runs all workspace tests, builds the workspace's documentation with warnings denied and
validates OpenSpec artifacts, and exits non-zero if any step fails. The steps that resolve
dependencies SHALL use the committed lock file unchanged. The command SHALL report how long
each step took.

#### Scenario: Gate passes on a healthy tree
- **WHEN** the gate command runs on a tree with no violations
- **THEN** it exits with status zero

#### Scenario: Gate reports the failing step
- **WHEN** any one of the five checks fails
- **THEN** the gate exits non-zero and identifies which check failed

#### Scenario: Broken documentation link
- **WHEN** a documentation comment links to an item that does not exist
- **THEN** the gate fails at the documentation step

#### Scenario: Lock file out of date
- **WHEN** a manifest asks for a dependency that the lock file does not hold
- **THEN** the gate fails without rewriting the lock file

#### Scenario: Durations
- **WHEN** the gate finishes, passing or failing
- **THEN** its output names each step that ran with the time it took

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`). Only the `vkxp` binary
may depend on both sides. As for the other dependency rules, development-only dependencies do
not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it

### Requirement: Build profiles
The workspace manifest SHALL define the build profiles: development and test builds optimise
dependencies and keep debug assertions and overflow checks in workspace crates; the release
profile uses link-time optimisation and one code generation unit and keeps line tables; a
profiling profile is the release profile with full debug information.

#### Scenario: Tests keep their checks
- **WHEN** a workspace test overflows an integer in a workspace crate
- **THEN** the test panics, as it did without the profiles

#### Scenario: Release executable
- **WHEN** `cargo build --release -p vkxp` runs
- **THEN** the executable is built with the release profile of the workspace manifest

### Requirement: Workspace lints
Every workspace crate SHALL inherit the workspace lints, which include, besides the existing
ones, `unreachable_pub`, `clippy::dbg_macro` and `clippy::todo`.

#### Scenario: Left-over debugging macro
- **WHEN** a `dbg!` call is committed in a workspace crate
- **THEN** the gate fails at the lint step

### Requirement: Large unit test modules of core live beside the module
In `vkxp-core`, a unit test module of more than about 300 lines SHALL be a file
`src/<module>/tests.rs` declared from the module with `#[cfg(test)] mod tests;`, instead of an
inline module.

#### Scenario: Frame extraction
- **WHEN** `crates/vkxp-core/src/extract.rs` is read
- **THEN** it holds the production code and the declaration of its test module, and the tests are in `crates/vkxp-core/src/extract/tests.rs`

#### Scenario: The same tests run
- **WHEN** the names printed by `cargo test -p vkxp-core -- --list` before and after the move are compared
- **THEN** the two lists are equal

### Requirement: Test doubles stay out of the release library
The test doubles and the demo guest of `vkxp-host` SHALL be compiled only for tests and
examples, behind a feature that the executable does not enable.

#### Scenario: Executable build
- **WHEN** the feature graph of the `vkxp` package is listed
- **THEN** the `testing` feature of `vkxp-host` is not in it

#### Scenario: Tests and examples
- **WHEN** `cargo test -p vkxp-host --locked` and `cargo build -p vkxp-host --examples --locked` run
- **THEN** both succeed without a feature flag on the command line, or with the one the gate passes

### Requirement: Dependency audit command
`cargo xtask audit` SHALL check the dependency graph against security advisories and a
committed licence allow-list. It SHALL NOT be a step of the merge gate. When its tool is not
installed it SHALL say how to install it and exit with a non-zero status.

#### Scenario: Tool missing
- **WHEN** `cargo xtask audit` runs without `cargo-deny` installed
- **THEN** it prints how to install it and exits with a non-zero status

#### Scenario: Gate unchanged
- **WHEN** the merge gate runs
- **THEN** it does not run the audit
