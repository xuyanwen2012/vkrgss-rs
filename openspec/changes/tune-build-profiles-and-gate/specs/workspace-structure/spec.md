# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
