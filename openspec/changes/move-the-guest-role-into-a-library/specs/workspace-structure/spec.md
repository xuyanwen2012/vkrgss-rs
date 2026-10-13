## MODIFIED Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkxp-proto`,
`vkxp-pixel`, `vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`, `vkxp-guest`, `vkxp-compose-ref`, `vkxp-vk`, `vkxp-audio`, `vkxp-host` and
the binary `vkxp`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`, `vkxp-guest`) SHALL depend, directly or transitively, on a host crate
(`vkxp-vk`, `vkxp-audio`, `vkxp-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest role
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-host`
- **THEN** the merge gate fails and names the offending path

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`, `vkxp-guest`). Only
the `vkxp` binary may depend on both sides. As for the other dependency rules,
development-only dependencies do not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it

## ADDED Requirements

### Requirement: Guest role library
What makes a process the guest of a program SHALL be in the library crate `vkxp-guest`:
the hidden option that names the program's socket, the attachment to that socket, the
copies of the process that are its states, the restriction the process puts on itself, and
the signal settings of a guest. `unsafe` in `vkxp-guest` SHALL be allowed only in the
modules of the copier and of the signals.

#### Scenario: The program holds no copy of the role
- **WHEN** `libc::fork`, `libc::prctl` and `landlock` are searched in `crates/vkxp/src` and in the `[dependencies]` of the manifest of `vkxp`
- **THEN** there is no match

#### Scenario: Unsafe elsewhere in the library
- **WHEN** `unsafe` is written in a module of `vkxp-guest` other than the copier and the signals
- **THEN** compilation fails

#### Scenario: Nothing else changes
- **WHEN** the tests of the workspace run after the move
- **THEN** every test that passed before passes, none is removed, and no test of `crates/vkxp/tests/end_to_end.rs` is changed

### Requirement: The guest role links no interpreter
`vkxp-guest` SHALL NOT depend, directly or transitively, on `vkxp-ruby-sys` or
`vkxp-binding`. The executable that links it SHALL give it the function that runs the game.

#### Scenario: Interpreter dependency is rejected
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-ruby-sys` or to `vkxp-binding`
- **THEN** the merge gate fails and names the offending path
