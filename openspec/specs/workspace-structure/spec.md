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

### Requirement: Direct dependencies follow their newest stable line
The workspace's direct dependencies SHALL allow the newest stable release line of each crate,
except where the change that last reviewed them recorded a reason to stay behind.

#### Scenario: The three upgraded crates
- **WHEN** `Cargo.toml` is read after this change
- **THEN** `sha2` allows 0.11 and `x11rb` allows 0.14, and `symphonia` allows 0.6 or the change's design records why it stays at 0.5

#### Scenario: Decoding unchanged
- **WHEN** the decoding tests of `vkxp-audio` run
- **THEN** they pass, with an expectation changed only where the design records an accepted difference

### Requirement: Package metadata
Every crate of the workspace SHALL have a description, the licence, the repository address
`https://github.com/xuyanwen2012/vkxp` and the workspace's version in its package metadata,
and SHALL NOT be publishable to a registry.

#### Scenario: Metadata of every crate
- **WHEN** `cargo metadata --no-deps` is read for the workspace
- **THEN** every package has a non-empty `description`, the `license` `MIT`, that `repository` and an empty `publish` list

### Requirement: Public documents
The repository SHALL have a `README.md` of at most 200 lines that states what vkxp is, its
status, how to build it and run a game, and the legal notice, and SHALL keep the manual in
topic files under `docs/`. It SHALL have `CHANGELOG.md`, `CONTRIBUTING.md` and
`THIRD-PARTY.md`. No tracked document SHALL state rules of one developer's machine.

#### Scenario: Landing page
- **WHEN** `README.md` is read
- **THEN** it has at most 200 lines and links to `docs/running.md`, `docs/building.md`, `docs/architecture.md`, `docs/development.md`, `docs/compatibility.md`, `CHANGELOG.md`, `CONTRIBUTING.md` and `THIRD-PARTY.md`

#### Scenario: Manual kept
- **WHEN** the headings of the README before this change are looked for in the files under `docs/`
- **THEN** each is found, apart from "Third-party source", whose text is in `THIRD-PARTY.md`

#### Scenario: Machine rules
- **WHEN** `AGENTS.md`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md` and the files under `docs/` are read
- **THEN** none of them names a graphics card of one developer's machine, the language of a chat or a directory outside the repository

### Requirement: Links of the documents resolve
A test SHALL fail when a relative link or image of `README.md`, `CHANGELOG.md`,
`CONTRIBUTING.md`, `THIRD-PARTY.md`, `AGENTS.md` or a file `docs/*.md` names a file or
directory that does not exist.

#### Scenario: All links valid
- **WHEN** the test runs on the repository
- **THEN** it passes

#### Scenario: Broken link
- **WHEN** the check is given a document that links to `docs/missing.md`
- **THEN** it reports that document and that target

### Requirement: Release package command
`cargo xtask dist` SHALL build the release executable and write under `target/dist/` an
archive named after the version and the target that holds the executable, the README, the
changelog, the licence, the third-party notice and the licence texts of the bundled source,
and beside it a SHA-256 checksum file that `sha256sum -c` accepts.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run
- **THEN** `target/dist/vkxp-0.1.0-x86_64-unknown-linux-gnu.tar.gz` unpacks to one directory holding an executable `vkxp`, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md`, `licenses/ruby-1.8.7/COPYING` and `licenses/oniguruma-5.9.6/COPYING`, and `sha256sum -c` on the checksum file passes

#### Scenario: Missing file
- **WHEN** a file of the list does not exist
- **THEN** the command fails and names it

### Requirement: Workflows
The repository SHALL have a workflow that runs the merge gate on a software Vulkan device
for every push to `main` and every pull request, and a workflow that for a version tag
builds the package with `cargo xtask dist` and publishes it as a GitHub release, and that
only builds it when started by hand.

#### Scenario: Gate in the workflow
- **WHEN** the continuous integration workflow file is read
- **THEN** it runs `cargo xtask gate` with `VK_LOADER_DRIVERS_SELECT` set to lavapipe

#### Scenario: Release only for a tag
- **WHEN** the release workflow file is read
- **THEN** the step that creates the release is conditional on a tag reference
