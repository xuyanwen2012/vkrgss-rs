## MODIFIED Requirements

### Requirement: Crate layout
The repository SHALL be a single Cargo workspace containing the crates `vkxp-proto`,
`vkxp-pixel`, `vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-ruby192-sys`, `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`,
`vkxp-guest-main-rgss3`, `vkxp-compose-ref`, `vkxp-vk`, `vkxp-audio`, `vkxp-host` and the
binaries `vkxp` and `vkxp-guest-rgss3`.

#### Scenario: Workspace builds from a clean checkout
- **WHEN** `cargo build --workspace` runs on a clean checkout with the pinned toolchain
- **THEN** every listed crate compiles without warnings, and the directory of the build holds the executables `vkxp` and `vkxp-guest-rgss3`

#### Scenario: The crates of the second interpreter on another target
- **WHEN** the workspace is built for a target other than `x86_64-unknown-linux-gnu`
- **THEN** `vkxp-ruby192-sys`, `vkxp-binding-rgss3` and `vkxp-guest-main-rgss3` build and are empty, `vkxp-guest-rgss3` builds an executable that only says it is not built for the target, and every other crate builds as before

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkxp-core`, `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`,
`vkxp-ruby192-sys`, `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`,
`vkxp-guest-main-rgss3`, `vkxp-guest-rgss3`) SHALL depend, directly or transitively, on a
host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest role
- **WHEN** `vkxp-guest` gains a dependency path to `vkxp-host`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The main of a guest
- **WHEN** `vkxp-guest-main` or `vkxp-guest-main-rgss3` gains a dependency path to `vkxp-host`, to `vkxp-vk` or to `vkxp-audio`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binding over the second interpreter
- **WHEN** `vkxp-binding-rgss3` gains a dependency path to `vkxp-host`, to `vkxp-vk` or to `vkxp-audio`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest executable of RGSS3
- **WHEN** the dependency rules are given a graph in which `vkxp-guest-rgss3` reaches `vkxp-host`, `vkxp-vk` or `vkxp-audio`
- **THEN** they report a violation that names the path, and over the real workspace they report none

### Requirement: Host crates do not depend on guest crates
No host crate (`vkxp-vk`, `vkxp-audio`, `vkxp-host`) and not the reference compositor
`vkxp-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkxp-core`,
`vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-ruby192-sys`, `vkxp-binding`,
`vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`, `vkxp-guest-main-rgss3`,
`vkxp-guest-rgss3`). Only the `vkxp` binary may depend on both sides. Development-only
dependencies do not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkxp` binary depends on a guest crate and on a host crate
- **THEN** the dependency check reports no violation for it

## ADDED Requirements

### Requirement: Second package of the main of a guest
`vkxp-guest-main-rgss3` SHALL compile the source of `vkxp-guest-main` over
`vkxp-binding-rgss3` and `vkxp-ruby192-sys` and SHALL hold no source of its own. Its
manifest SHALL declare the dependencies of `vkxp-guest-main` with those two packages in the
place of `vkxp-binding` and `vkxp-ruby-sys`. The source SHALL name no item that only one of
the interpreter crates has, and no `cfg` or cargo feature SHALL tell the interpreters apart.

#### Scenario: Two libraries from one source
- **WHEN** `cargo test -p vkxp-guest-main -p vkxp-guest-main-rgss3 --locked` runs on `x86_64-unknown-linux-gnu`
- **THEN** both packages build without a warning and run the unit tests of `crates/vkxp-guest-main/src`, and `crates/vkxp-guest-main-rgss3` has no `src` directory

#### Scenario: A dependency in one package only
- **WHEN** one of the two manifests gains a dependency, a feature of one or an optional flag that the other lacks
- **THEN** a test of `xtask` fails and names the dependency

#### Scenario: A test file without its counterpart
- **WHEN** `crates/vkxp-guest-main/tests` holds a `.rs` file that `crates/vkxp-guest-main-rgss3/tests` lacks
- **THEN** a test of `xtask` fails and names the file

#### Scenario: One interpreter each
- **WHEN** the dependency rules are checked over the workspace
- **THEN** `vkxp-guest-main-rgss3` and `vkxp-guest-rgss3` reach `vkxp-ruby192-sys` and not `vkxp-ruby-sys`, and `vkxp-guest-main` and `vkxp` reach `vkxp-ruby-sys` and not `vkxp-ruby192-sys`

#### Scenario: Documentation once
- **WHEN** the documentation of the workspace is built
- **THEN** it has no library `vkxp_guest_main_rgss3`

### Requirement: Guest executable of RGSS3
The package `vkxp-guest-rgss3` SHALL be one binary of that name that depends on
`vkxp-guest-main-rgss3` and on no other package of the workspace directly. Its `main` SHALL
call the function of the glue that makes an executable a guest process, with the package
version, and SHALL do nothing else as a guest. The package SHALL take the version of the
workspace, as `vkxp` does.

#### Scenario: Direct dependencies
- **WHEN** the dependencies that the manifest of `vkxp-guest-rgss3` declares, its development dependencies left out, are listed
- **THEN** `vkxp-guest-main-rgss3` is the only package of the workspace among them

#### Scenario: One version
- **WHEN** `cargo metadata --locked --no-deps` is read
- **THEN** the packages `vkxp` and `vkxp-guest-rgss3` have the same version

#### Scenario: Nothing of the host is linked
- **WHEN** the packages that `vkxp-guest-rgss3` reaches are listed
- **THEN** none of `vkxp-host`, `vkxp-vk`, `vkxp-audio`, `ash`, `winit` and `cpal` is among them

### Requirement: Tests of the program reach the guest executable beside it
The tests of `crates/vkxp` that run a game of RGSS3 SHALL be in a test file of their own and
SHALL let the program find `vkxp-guest-rgss3` as a player's does. `cargo test --workspace`
and `cargo test -p vkxp -p vkxp-guest-rgss3` SHALL build that executable from the sources as
they are before a test runs. A test that finds none SHALL fail and name that command.

#### Scenario: The gate
- **WHEN** `cargo test --workspace --locked` runs on a clean checkout with the loader restricted
- **THEN** the tests of `crates/vkxp` that run a game of RGSS3 pass

#### Scenario: No guest executable
- **WHEN** such a test runs while no file `vkxp-guest-rgss3` is beside the executable of the program
- **THEN** it fails before it starts a child, with a message that names `cargo test -p vkxp -p vkxp-guest-rgss3`

#### Scenario: The stress step of Ruby 1.8.7
- **WHEN** `cargo test -p vkxp --locked --test end_to_end` runs with the loader restricted and no guest executable of RGSS3 built
- **THEN** it passes: no test of that file runs a game of RGSS3

#### Scenario: Another target
- **WHEN** the test file of RGSS3 is built for a target other than `x86_64-unknown-linux-gnu`
- **THEN** it holds no test

### Requirement: Release package holds both executables
`cargo xtask dist` SHALL build `vkxp` and `vkxp-guest-rgss3` for release in one build with
libcrypt linked statically into both, SHALL fail when either needs a shared libcrypt, and
SHALL put both executables side by side into the archive, with the licence texts of the
bundled Ruby 1.9.2.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run
- **THEN** the archive unpacks to one directory that holds the executables `vkxp` and `vkxp-guest-rgss3`, the files it held before, and `licenses/ruby-1.9.2/COPYING`, `licenses/ruby-1.9.2/GPL` and `licenses/ruby-1.9.2/LEGAL`

#### Scenario: Missing guest executable
- **WHEN** the package is put together from a tree without `target/release/vkxp-guest-rgss3`
- **THEN** the command fails and names that file

#### Scenario: Shared libcrypt in the guest executable
- **WHEN** the check of needed libraries is given `libcrypt.so.1` for the guest executable
- **THEN** the command fails and names the library and the executable

#### Scenario: The two files are enough
- **WHEN** the executables `vkxp` and `vkxp-guest-rgss3` of a build are copied into an empty directory and the `vkxp` there runs a generated game of RGSS3 headless
- **THEN** the exit status is 0
