## MODIFIED Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkxp-proto`,
`vkxp-pixel`, `vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-ruby192-sys`, `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`,
`vkxp-compose-ref`, `vkxp-vk`, `vkxp-audio`, `vkxp-host` and the binary `vkxp`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings

#### Scenario: The crates of the second interpreter on another target
- **WHEN** the workspace is built for a target other than `x86_64-unknown-linux-gnu`
- **THEN** `vkxp-ruby192-sys` and `vkxp-binding-rgss3` build and are empty, and every other crate builds as before

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-ruby192-sys`, `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`)
SHALL depend, directly or transitively, on a host crate (`vkxp-vk`, `vkxp-audio`,
`vkxp-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest role
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-host`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The main of a guest
- **WHEN** `vkxp-guest-main` gains a dependency path to `vkxp-host`, to `vkxp-vk` or to `vkxp-audio`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binding over the second interpreter
- **WHEN** `vkxp-binding-rgss3` gains a dependency path to `vkxp-host`, to `vkxp-vk` or to `vkxp-audio`
- **THEN** the merge gate fails and names the offending path

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-ruby192-sys`, `vkxp-binding`,
`vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`). Only the `vkxp` binary may depend
on both sides. As for the other
dependency rules, development-only dependencies do not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it

### Requirement: The guest role links no interpreter
`vkxp-guest` SHALL NOT depend, directly or transitively, on `vkxp-ruby-sys`,
`vkxp-ruby192-sys`, `vkxp-binding` or `vkxp-binding-rgss3`. The executable that links it
SHALL give it the function that runs the game.

#### Scenario: Interpreter dependency is rejected
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-ruby-sys`, to `vkxp-ruby192-sys`, to `vkxp-binding` or to `vkxp-binding-rgss3`
- **THEN** the merge gate fails and names the offending path
