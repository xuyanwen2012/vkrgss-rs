# workspace-structure Specification

## Purpose
Defines the Cargo workspace layout, the dependency-direction and purity rules between crates, and the single merge gate that enforces them.

## Requirements

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

### Requirement: Guest crates do not depend on host crates
No guest crate (`vkrgss-core`, `vkrgss-image`, `vkrgss-text`, `vkrgss-vfs`, `vkrgss-ruby-sys`,
`vkrgss-ruby192-sys`, `vkrgss-binding`, `vkrgss-binding-rgss3`, `vkrgss-guest`, `vkrgss-guest-main`,
`vkrgss-guest-main-rgss3`, `vkrgss-guest-rgss3`) SHALL depend, directly or transitively, on a
host crate (`vkrgss-vk`, `vkrgss-audio`, `vkrgss-host`).

#### Scenario: Forbidden dependency is rejected
- **WHEN** a guest crate gains a dependency path to a host crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest role
- **WHEN** `vkrgss-guest` gains a dependency path to `vkrgss-host`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The main of a guest
- **WHEN** `vkrgss-guest-main` or `vkrgss-guest-main-rgss3` gains a dependency path to `vkrgss-host`, to `vkrgss-vk` or to `vkrgss-audio`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binding over the second interpreter
- **WHEN** `vkrgss-binding-rgss3` gains a dependency path to `vkrgss-host`, to `vkrgss-vk` or to `vkrgss-audio`
- **THEN** the merge gate fails and names the offending path

#### Scenario: The guest executable of RGSS3
- **WHEN** the dependency rules are given a graph in which `vkrgss-guest-rgss3` reaches `vkrgss-host`, `vkrgss-vk` or `vkrgss-audio`
- **THEN** they report a violation that names the path, and over the real workspace they report none

### Requirement: Core stays pure
`vkrgss-core` SHALL contain no `unsafe` code and SHALL NOT depend on Ruby, Vulkan, windowing,
audio or file-system crates.

#### Scenario: Unsafe code in core is rejected
- **WHEN** `unsafe` is introduced in `vkrgss-core`
- **THEN** compilation fails

#### Scenario: Impure dependency in core is rejected
- **WHEN** `vkrgss-core` gains a dependency outside its allow-list
- **THEN** the merge gate fails and names the dependency

### Requirement: Single merge gate
The repository SHALL provide one command that checks formatting, lints with warnings denied,
runs all workspace tests, runs the Vulkan tests again without the validation layer, builds the workspace's documentation with warnings denied and
validates OpenSpec artifacts, and exits non-zero if any step fails. The steps that resolve
dependencies SHALL use the committed lock file unchanged. The command SHALL report how long
each step took.

#### Scenario: Gate passes on a healthy tree
- **WHEN** the gate command runs on a tree with no violations
- **THEN** it exits with status zero

#### Scenario: Gate reports the failing step
- **WHEN** any one of the six checks fails
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

#### Scenario: Fault hidden by the validation layer
- **WHEN** a test of the Vulkan backend passes only because the validation layer is loaded
- **THEN** the gate fails at the step that runs without the layer

#### Scenario: Two failing test binaries
- **WHEN** tests of two crates fail
- **THEN** the output of the gate names both

### Requirement: Host crates do not depend on guest crates
No host crate (`vkrgss-vk`, `vkrgss-audio`, `vkrgss-host`) and not the reference compositor
`vkrgss-compose-ref` SHALL depend, directly or transitively, on a guest crate (`vkrgss-core`,
`vkrgss-image`, `vkrgss-text`, `vkrgss-vfs`, `vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-binding`,
`vkrgss-binding-rgss3`, `vkrgss-guest`, `vkrgss-guest-main`, `vkrgss-guest-main-rgss3`,
`vkrgss-guest-rgss3`). Only the `vkrgss` binary may depend on both sides. Development-only
dependencies do not count.

#### Scenario: Forbidden dependency is rejected
- **WHEN** a host crate or the reference compositor gains a dependency path to a guest crate
- **THEN** the merge gate fails and names the offending path

#### Scenario: The binary may join both sides
- **WHEN** the `vkrgss` binary depends on a guest crate and on a host crate
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
- **WHEN** `cargo build --release -p vkrgss` runs
- **THEN** the executable is built with the release profile of the workspace manifest

### Requirement: Workspace lints
Every workspace crate SHALL inherit the workspace lints, which include, besides the existing
ones, `unreachable_pub`, `clippy::dbg_macro` and `clippy::todo`.

#### Scenario: Left-over debugging macro
- **WHEN** a `dbg!` call is committed in a workspace crate
- **THEN** the gate fails at the lint step

### Requirement: Large unit test modules of core live beside the module
In `vkrgss-core`, a unit test module of more than about 300 lines SHALL be a file
`src/<module>/tests.rs` declared from the module with `#[cfg(test)] mod tests;`, instead of an
inline module.

#### Scenario: Frame extraction
- **WHEN** `crates/vkrgss-core/src/extract.rs` is read
- **THEN** it holds the production code and the declaration of its test module, and the tests are in `crates/vkrgss-core/src/extract/tests.rs`

#### Scenario: The same tests run
- **WHEN** the names printed by `cargo test -p vkrgss-core -- --list` before and after the move are compared
- **THEN** the two lists are equal

### Requirement: Test doubles stay out of the release library
The test doubles and the demo guest of `vkrgss-host` SHALL be compiled only for tests and
examples, behind a feature that the executable does not enable.

#### Scenario: Executable build
- **WHEN** the feature graph of the `vkrgss` package is listed
- **THEN** the `testing` feature of `vkrgss-host` is not in it

#### Scenario: Tests and examples
- **WHEN** `cargo test -p vkrgss-host --locked` and `cargo build -p vkrgss-host --examples --locked` run
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
- **WHEN** the decoding tests of `vkrgss-audio` run
- **THEN** they pass, with an expectation changed only where the design records an accepted difference

### Requirement: Package metadata
Every crate of the workspace SHALL have a description, the licence, the repository address
`https://github.com/xuyanwen2012/vkrgss-rs` and the workspace's version in its package metadata,
and SHALL NOT be publishable to a registry.

#### Scenario: Metadata of every crate
- **WHEN** `cargo metadata --no-deps` is read for the workspace
- **THEN** every package has a non-empty `description`, the `license` `MIT`, that `repository` and an empty `publish` list

### Requirement: Public documents
The repository SHALL have a `README.md` of at most 200 lines that states what vkrgss is, its
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
- **WHEN** `cargo xtask dist` has run on x86_64 Linux
- **THEN** `target/dist/vkrgss-0.1.0-x86_64-unknown-linux-gnu.tar.gz` unpacks to one directory holding an executable `vkrgss`, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md`, `licenses/ruby-1.8.7/COPYING` and `licenses/oniguruma-5.9.6/COPYING`, and `sha256sum -c` on the checksum file passes

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

### Requirement: Pinned inputs of the workflows
The workflows SHALL refer to actions by commit hash, install tools at fixed versions, name
the C compiler, and cancel a run in progress only for a pull request.

#### Scenario: Pinned inputs
- **WHEN** the workflow files are read
- **THEN** every `uses:` names a 40-character commit hash, `@fission-ai/openspec` is installed with a version, and the jobs that build set `CC`

#### Scenario: Pushes to main are not cancelled
- **WHEN** the continuous integration workflow file is read
- **THEN** `cancel-in-progress` is true only for pull requests

### Requirement: Scheduled checks
A scheduled workflow SHALL run the dependency audit, the collector stress, the sanitized
suites and a short run of each fuzz target, without blocking merges.

#### Scenario: Scheduled checks
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** it runs on a schedule and by hand, and its jobs run `cargo xtask audit`, `cargo xtask stress`, the sanitized suites and `cargo xtask fuzz` for each target

### Requirement: The release package does not depend on the name of libcrypt
The executable in the package that `cargo xtask dist` writes SHALL NOT need a libcrypt
shared library at run time. Other builds SHALL link it as before. When the static library
is not installed, `cargo xtask dist` SHALL fail before building and name it.

#### Scenario: Needed libraries of the package
- **WHEN** the dynamic section of the packaged executable lists `libcrypt.so.1`
- **THEN** `cargo xtask dist` fails and names that library

#### Scenario: Static library missing
- **WHEN** `cargo xtask dist` runs on a machine without `libcrypt.a`
- **THEN** it fails before building, with a message that names the file and the packages that provide it

#### Scenario: Development build
- **WHEN** the workspace is built without `VKRGSS_STATIC_CRYPT`
- **THEN** libcrypt is linked dynamically and no static library is needed

### Requirement: aarch64 job
The CI workflow SHALL have a job that runs the gate on an aarch64 runner, and its failure
SHALL fail the workflow. The documents SHALL name x86_64 and aarch64 Linux as supported.

#### Scenario: Job is present and may fail
- **WHEN** `.github/workflows/ci.yml` is read
- **THEN** it has a job on `ubuntu-24.04-arm` with the steps of the gate job and without `continue-on-error`, so its failure is not allowed

#### Scenario: Documents
- **WHEN** `docs/compatibility.md` is read
- **THEN** it names x86_64 and aarch64 Linux as supported

### Requirement: Guest role library
What makes a process the guest of a program SHALL be in the library crate `vkrgss-guest`:
the hidden option that names the guest record, that record and its encoding, the
subscriber of the diagnostics, the attachment to the program's socket, the copies of the
process that are its states, the restriction the process puts on itself, and the signal
settings of a guest. `unsafe` in `vkrgss-guest` SHALL be allowed only in the modules of the
copier and of the signals.

#### Scenario: The program holds no copy of the role
- **WHEN** `libc::fork`, `libc::prctl` and `landlock` are searched in `crates/vkrgss/src` and in the `[dependencies]` of the manifest of `vkrgss`
- **THEN** there is no match

#### Scenario: Unsafe elsewhere in the library
- **WHEN** `unsafe` is written in a module of `vkrgss-guest` other than the copier and the signals
- **THEN** compilation fails

#### Scenario: Nothing else changes
- **WHEN** the tests of the workspace run after code has moved into the library
- **THEN** every test that passed before passes, none is removed apart from a test of something that no longer exists, and no test that exists in `crates/vkrgss/tests/end_to_end.rs` is altered

### Requirement: The guest role links no interpreter
`vkrgss-guest` SHALL NOT depend, directly or transitively, on `vkrgss-ruby-sys`,
`vkrgss-ruby192-sys`, `vkrgss-binding` or `vkrgss-binding-rgss3`. The executable that links it
SHALL give it the function that runs the game.

#### Scenario: Interpreter dependency is rejected
- **WHEN** `vkrgss-guest` gains a dependency path to `vkrgss-ruby-sys`, to `vkrgss-ruby192-sys`, to `vkrgss-binding` or to `vkrgss-binding-rgss3`
- **THEN** the merge gate fails and names the offending path

### Requirement: Guest main library
What a guest does with its binding and its interpreter SHALL be in the library crate
`vkrgss-guest-main`: what a guest is set up with, the text backend over the font engine, the
guest on a thread, and one function that makes the executable calling it a guest process
when its first argument is the hidden option. The crate SHALL forbid `unsafe`. The program
SHALL call that function before it reads its command line, and SHALL hold no copy of this
code.

#### Scenario: The program names no interpreter
- **WHEN** the dependencies that the manifest of `vkrgss` declares, its development dependencies left out, are listed
- **THEN** neither `vkrgss-ruby-sys` nor `vkrgss-text` is among them

#### Scenario: Unsafe in the main of a guest
- **WHEN** `unsafe` is written in `vkrgss-guest-main`
- **THEN** compilation fails

#### Scenario: XP and VX as before
- **WHEN** the tests of the workspace run after the change
- **THEN** every test that passed before passes under its name, apart from the crate or module in front of a moved one and from the two tests of the value `SEED=SOCKET` of the hidden option, which are gone

### Requirement: Second package of the main of a guest
`vkrgss-guest-main-rgss3` SHALL compile the source of `vkrgss-guest-main` over
`vkrgss-binding-rgss3` and `vkrgss-ruby192-sys` and SHALL hold no source of its own. Its
manifest SHALL declare the dependencies of `vkrgss-guest-main` with those two packages in the
place of `vkrgss-binding` and `vkrgss-ruby-sys`. The source SHALL name no item that only one of
the interpreter crates has, and no `cfg` or cargo feature SHALL tell the interpreters apart.

#### Scenario: Two libraries from one source
- **WHEN** `cargo test -p vkrgss-guest-main -p vkrgss-guest-main-rgss3 --locked` runs on `x86_64-unknown-linux-gnu`
- **THEN** both packages build without a warning and run the unit tests of `crates/vkrgss-guest-main/src`, and `crates/vkrgss-guest-main-rgss3` has no `src` directory

#### Scenario: A dependency in one package only
- **WHEN** one of the two manifests gains a dependency, a feature of one or an optional flag that the other lacks
- **THEN** a test of `xtask` fails and names the dependency

#### Scenario: A test file without its counterpart
- **WHEN** `crates/vkrgss-guest-main/tests` holds a `.rs` file that `crates/vkrgss-guest-main-rgss3/tests` lacks
- **THEN** a test of `xtask` fails and names the file

#### Scenario: One interpreter each
- **WHEN** the dependency rules are checked over the workspace
- **THEN** `vkrgss-guest-main-rgss3` and `vkrgss-guest-rgss3` reach `vkrgss-ruby192-sys` and not `vkrgss-ruby-sys`, and `vkrgss-guest-main` and `vkrgss` reach `vkrgss-ruby-sys` and not `vkrgss-ruby192-sys`

#### Scenario: Documentation once
- **WHEN** the documentation of the workspace is built
- **THEN** it has no library `vkrgss_guest_main_rgss3`

### Requirement: Guest executable of RGSS3
The package `vkrgss-guest-rgss3` SHALL be one binary of that name that depends on
`vkrgss-guest-main-rgss3` and on no other package of the workspace directly. Its `main` SHALL
call the function of the glue that makes an executable a guest process, with the package
version, and SHALL do nothing else as a guest. The package SHALL take the version of the
workspace, as `vkrgss` does.

#### Scenario: Direct dependencies
- **WHEN** the dependencies that the manifest of `vkrgss-guest-rgss3` declares, its development dependencies left out, are listed
- **THEN** `vkrgss-guest-main-rgss3` is the only package of the workspace among them

#### Scenario: One version
- **WHEN** `cargo metadata --locked --no-deps` is read
- **THEN** the packages `vkrgss` and `vkrgss-guest-rgss3` have the same version

#### Scenario: Nothing of the host is linked
- **WHEN** the packages that `vkrgss-guest-rgss3` reaches are listed
- **THEN** none of `vkrgss-host`, `vkrgss-vk`, `vkrgss-audio`, `ash`, `winit` and `cpal` is among them

### Requirement: Tests of the program reach the guest executable beside it
The tests of `crates/vkrgss` that run a game of RGSS3 SHALL be in a test file of their own and
SHALL let the program find `vkrgss-guest-rgss3` as a player's does. `cargo test --workspace`
and `cargo test -p vkrgss -p vkrgss-guest-rgss3` SHALL build that executable from the sources as
they are before a test runs. A test that finds none SHALL fail and name that command.

#### Scenario: The gate
- **WHEN** `cargo test --workspace --locked` runs on a clean checkout with the loader restricted
- **THEN** the tests of `crates/vkrgss` that run a game of RGSS3 pass

#### Scenario: No guest executable
- **WHEN** such a test runs while no file `vkrgss-guest-rgss3` is beside the executable of the program
- **THEN** it fails before it starts a child, with a message that names `cargo test -p vkrgss -p vkrgss-guest-rgss3`

#### Scenario: The stress step of Ruby 1.8.7
- **WHEN** `cargo test -p vkrgss --locked --test end_to_end` runs with the loader restricted and no guest executable of RGSS3 built
- **THEN** it passes: no test of that file runs a game of RGSS3

#### Scenario: Another target
- **WHEN** the test file of RGSS3 is built for a target other than `x86_64-unknown-linux-gnu`
- **THEN** it holds no test

### Requirement: Release package holds the executables
`cargo xtask dist` SHALL build `vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3` for
release in one build with libcrypt linked statically into each, SHALL fail when any needs a
shared libcrypt, and SHALL put the executables side by side into the archive, with the licence texts of the
bundled Ruby 1.9.2.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run
- **THEN** the archive unpacks to one directory that holds the executables `vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3`, the files it held before, and `licenses/ruby-1.9.2/COPYING`, `licenses/ruby-1.9.2/GPL` and `licenses/ruby-1.9.2/LEGAL`

#### Scenario: Missing guest executable
- **WHEN** the package is put together from a tree without `target/release/vkrgss-guest-rgss3`, or without `target/release/vkrgss-guest-rgss12`
- **THEN** the command fails and names that file

#### Scenario: Shared libcrypt in the guest executable
- **WHEN** the check of needed libraries is given `libcrypt.so.1` for the guest executable
- **THEN** the command fails and names the library and the executable

#### Scenario: The two files are enough
- **WHEN** the executables `vkrgss` and `vkrgss-guest-rgss3` of a build are copied into an empty directory and the `vkrgss` there runs a generated game of RGSS3 headless
- **THEN** the exit status is 0

#### Scenario: The three files are enough
- **WHEN** the three executables of a build are copied into an empty directory and the `vkrgss` there runs a generated game of RGSS1, one of RGSS2 and one of RGSS3 headless with `--guest process`
- **THEN** each exit status is 0

### Requirement: Name of the project
The project SHALL be named vkrgss: the program `vkrgss`, the guest executable
`vkrgss-guest-rgss3`, the crates `vkrgss` and `vkrgss-*`, the environment variables
`VKRGSS_*` and the repository `https://github.com/xuyanwen2012/vkrgss-rs`. The README SHALL
say once that versions 0.1.0 to 0.3.0 were released under the name vkxp.

#### Scenario: Packages
- **WHEN** `cargo metadata --format-version 1 --no-deps --locked` is read
- **THEN** every package but `xtask` is named `vkrgss` or starts with `vkrgss-`, and none is named with `vkxp`

#### Scenario: Executables
- **WHEN** `cargo build --workspace --locked` has run on a clean checkout
- **THEN** the build directory holds the executables `vkrgss` and `vkrgss-guest-rgss3`, and none whose name contains `vkxp`

#### Scenario: Version line
- **WHEN** `vkrgss --version` is run
- **THEN** the standard output is `vkrgss` and the version of the package on one line

#### Scenario: The former name in the README
- **WHEN** `README.md` is read
- **THEN** its title is `vkrgss`, and outside the alternative text of the banner `vkxp` appears only in the one sentence that says versions 0.1.0 to 0.3.0 were released under that name

### Requirement: The former name only where it is kept
A test of `xtask` SHALL fail when a tracked file outside `openspec/` holds `vkxp`, in any
case, in its path or its text, at a place that the committed allow-list
`xtask/tests/former-name.allow` does not allow, and SHALL name each file and line. The
allow-list SHALL allow only what `design.md` of `rename-to-vkrgss` keeps, each entry with a
path pattern, a text pattern and its reason.

#### Scenario: The repository
- **WHEN** the test runs on the repository
- **THEN** it passes

#### Scenario: A new mention
- **WHEN** the check is given the tracked file `docs/running.md` with a line 7 that names `vkxp`
- **THEN** it reports `docs/running.md:7`

#### Scenario: A path
- **WHEN** the check is given the tracked path `crates/vkxp-core/src/lib.rs`, whose text does not name `vkxp`
- **THEN** it reports that path

#### Scenario: An identifier of the vendored interpreters
- **WHEN** the check is given a line of `crates/vkrgss-ruby-sys/src/ffi.rs` that names `ruby_vkxp_stack_size`
- **THEN** it reports nothing

#### Scenario: A vendored source
- **WHEN** the check is given a line of `third_party/ruby-1.8.7/gc.c` that names `vkxp`
- **THEN** it reports nothing

#### Scenario: The document of a vendored tree
- **WHEN** the check is given the tracked path `third_party/ruby-1.8.7/README.vkxp.md`
- **THEN** it reports that path

### Requirement: Parts of the gate
`cargo xtask gate --job <name>` SHALL run one named part of the gate. The parts together
SHALL run exactly the checks of `cargo xtask gate`, which SHALL be unchanged.

#### Scenario: Union of the parts
- **WHEN** the test of `xtask` compares the parts with the full gate
- **THEN** every check, package and test target of the gate is in exactly one part

#### Scenario: Unknown part
- **WHEN** `cargo xtask gate --job nothing` runs
- **THEN** it fails and lists the names of the parts

### Requirement: Parallel merge checks in CI
The CI workflow SHALL run each part of the gate as a job of its own, with a compiler cache
for C and Rust, and the nightly workflow SHALL open or update an issue when one of its jobs
fails.

#### Scenario: One job per part
- **WHEN** `.github/workflows/ci.yml` is read
- **THEN** each part has a job that runs `cargo xtask gate --job` with its name

#### Scenario: Nightly failure
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** a job that runs only when another failed opens or comments on an issue, and only that job may write issues

### Requirement: Guest executable of RGSS1 and RGSS2
The package `vkrgss-guest-rgss12` SHALL be one binary of that name that depends on
`vkrgss-guest-main` and on no other package of the workspace directly. Its `main` SHALL call
the function of the glue that makes an executable a guest process, with the package
version, and SHALL do nothing else as a guest. The package SHALL take the version of the
workspace.

#### Scenario: Direct dependencies
- **WHEN** the dependencies that the manifest of `vkrgss-guest-rgss12` declares, its development dependencies left out, are listed
- **THEN** `vkrgss-guest-main` is the only package of the workspace among them

#### Scenario: Nothing of the host is linked
- **WHEN** the packages that `vkrgss-guest-rgss12` reaches are listed
- **THEN** none of `vkrgss-host`, `vkrgss-vk`, `vkrgss-audio`, `ash`, `winit` and `cpal` is among them

#### Scenario: Started by anything else
- **WHEN** `vkrgss-guest-rgss12` is started without the hidden option of the guest role
- **THEN** it says what it is and that `vkrgss <game directory>` starts it, and exits with status 2

### Requirement: The nightly fuzz job keeps its findings
The fuzz job of `nightly.yml` SHALL upload the inputs a target put aside when the job fails,
and the dependency audit SHALL run with a version of `cargo-deny` that reads the current
advisory database.

#### Scenario: Upload on failure
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** the fuzz job has a step that runs on failure and uploads `target/tmp/fuzz/<target>/artifacts/` with a pinned action

#### Scenario: Version of cargo-deny
- **WHEN** the audit job is read
- **THEN** it installs `cargo-deny` at 0.20.2

### Requirement: Target of the release package
The target of the package SHALL be that of the machine `cargo xtask dist` runs on,
`x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu`; on any other machine the command
SHALL fail and say so.

#### Scenario: Package on aarch64
- **WHEN** `cargo xtask dist` has run on aarch64 Linux
- **THEN** the archive is named `vkrgss-<version>-aarch64-unknown-linux-gnu.tar.gz` and holds the same files

### Requirement: Both Linux packages in a release
The release workflow SHALL build the Linux package on an x86_64 and on an aarch64 runner, and a
release SHALL carry both archives with their checksum files.

#### Scenario: Both packages in a release
- **WHEN** the release workflow file is read
- **THEN** it builds the package on `ubuntu-24.04` and `ubuntu-24.04-arm` and passes both archives and checksum files to the step that creates the release

### Requirement: Fuzz targets of the file readers
`cargo xtask fuzz` SHALL also run three targets, `jpeg`, `bmp` and `rgssad`, that feed
arbitrary bytes to the JPEG decoder, the BMP decoder and the RGSSAD reader of both versions.
A target SHALL fail on a panic, on an allocation beyond the stated bounds, or on a decode that
returns part of an image. The `fuzz` job of `nightly.yml` SHALL run them.

#### Scenario: A short run
- **WHEN** `cargo xtask fuzz jpeg --seconds 30` is run
- **THEN** the target runs for about 30 seconds and ends with status 0 when nothing failed

#### Scenario: Nightly
- **WHEN** `.github/workflows/nightly.yml` is read
- **THEN** the matrix of the `fuzz` job names `jpeg`, `bmp` and `rgssad`

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
