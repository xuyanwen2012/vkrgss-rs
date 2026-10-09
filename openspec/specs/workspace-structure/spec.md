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
runs all workspace tests and validates OpenSpec artifacts, and exits non-zero if any step fails.

#### Scenario: Gate passes on a healthy tree
- **WHEN** the gate command runs on a tree with no violations
- **THEN** it exits with status zero

#### Scenario: Gate reports the failing step
- **WHEN** any one of the four checks fails
- **THEN** the gate exits non-zero and identifies which check failed

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
