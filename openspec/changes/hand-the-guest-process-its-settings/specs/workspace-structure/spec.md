## MODIFIED Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkxp-proto`,
`vkxp-pixel`, `vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`, `vkxp-guest`, `vkxp-guest-main`, `vkxp-compose-ref`, `vkxp-vk`,
`vkxp-audio`, `vkxp-host` and the binary `vkxp`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-binding`, `vkxp-guest`, `vkxp-guest-main`) SHALL depend, directly or transitively, on
a host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest role
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-host`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The main of a guest
- **WHEN** `vkxp-guest-main` gains a dependency path to `vkxp-host`, to `vkxp-vk` or to `vkxp-audio`
- **THEN** the merge gate fails and names the offending path

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding`, `vkxp-guest`,
`vkxp-guest-main`). Only the `vkxp` binary may depend on both sides. As for the other
dependency rules, development-only dependencies do not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it

### Requirement: Guest role library
What makes a process the guest of a program SHALL be in the library crate `vkxp-guest`:
the hidden option that names the guest record, that record and its encoding, the
subscriber of the diagnostics, the attachment to the program's socket, the copies of the
process that are its states, the restriction the process puts on itself, and the signal
settings of a guest. `unsafe` in `vkxp-guest` SHALL be allowed only in the modules of the
copier and of the signals.

#### Scenario: The program holds no copy of the role
- **WHEN** `libc::fork`, `libc::prctl` and `landlock` are searched in `crates/vkxp/src` and in the `[dependencies]` of the manifest of `vkxp`
- **THEN** there is no match

#### Scenario: Unsafe elsewhere in the library
- **WHEN** `unsafe` is written in a module of `vkxp-guest` other than the copier and the signals
- **THEN** compilation fails

#### Scenario: Nothing else changes
- **WHEN** the tests of the workspace run after code has moved into the library
- **THEN** every test that passed before passes, none is removed apart from a test of something that no longer exists, and no test that exists in `crates/vkxp/tests/end_to_end.rs` is altered

## ADDED Requirements

### Requirement: Guest main library
What a guest does with its binding and its interpreter SHALL be in the library crate
`vkxp-guest-main`: what a guest is set up with, the text backend over the font engine, the
guest on a thread, and one function that makes the executable calling it a guest process
when its first argument is the hidden option. The crate SHALL forbid `unsafe`. The program
SHALL call that function before it reads its command line, and SHALL hold no copy of this
code.

#### Scenario: The program names no interpreter
- **WHEN** the dependencies that the manifest of `vkxp` declares, its development dependencies left out, are listed
- **THEN** neither `vkxp-ruby-sys` nor `vkxp-text` is among them

#### Scenario: Unsafe in the main of a guest
- **WHEN** `unsafe` is written in `vkxp-guest-main`
- **THEN** compilation fails

#### Scenario: XP and VX as before
- **WHEN** the tests of the workspace run after the change
- **THEN** every test that passed before passes under its name, apart from the crate or module in front of a moved one and from the two tests of the value `SEED=SOCKET` of the hidden option, which are gone
