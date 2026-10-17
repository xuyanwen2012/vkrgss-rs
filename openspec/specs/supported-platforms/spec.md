# supported-platforms Specification

## Purpose
Defines which platforms vkrgss supports and plans, in what order, and how each is accepted.

## Requirements

### Requirement: Planned platforms and their order
`docs/decisions.md` SHALL name Linux x86_64 as the platform supported now and SHALL list the
planned platforms in the order ARM Linux, macOS, Android, Windows, each as a milestone row
P1 to P4 whose status follows recorded evidence.

#### Scenario: Scope
- **WHEN** the section "Goal and scope" of `docs/decisions.md` is read
- **THEN** it names Linux x86_64 as supported now and the four planned platforms in that order

#### Scenario: Milestone rows
- **WHEN** the section "Milestones" is read
- **THEN** it has rows P1 to P4, each "Not started", with its outcome and acceptance, below the rows A0 to A3, which are unchanged

### Requirement: Acceptance of a platform
`docs/decisions.md` SHALL state for each planned platform how it is accepted; for ARM Linux,
that frames and saves on aarch64 are identical byte for byte to those of x86_64 for the same
games, frames and input.

#### Scenario: ARM Linux
- **WHEN** the row P1 is read
- **THEN** its acceptance compares aarch64 with x86_64 byte for byte, frames and saves

#### Scenario: Windows without save states
- **WHEN** the decisions on Windows are read
- **THEN** they say that save states are not supported there and why

### Requirement: Recorded decisions of the grilling of 2026-10-15
`docs/decisions.md` SHALL record the decisions of `design.md` of this change on floats in
saves, Vulkan, display, audio, Linux packages, CI, test devices and the order of work.

#### Scenario: Each decision is there
- **WHEN** `docs/decisions.md` is read against the sections of `design.md` of this change
- **THEN** each decision is found, in the section where it belongs

### Requirement: The interpreters build and are tested on Android
Both interpreter crates SHALL build for `aarch64-linux-android` at API level 34 or later,
and their tests SHALL pass on a phone over adb, except those that need the host, each of
which names the reason. A build below API level 34 SHALL fail with a message that names
the minimum.

#### Scenario: Upstream suites on a phone
- **WHEN** the upstream suites of both interpreters run on the Pixel 7a and on the Galaxy S24+ through the adb runner
- **THEN** they pass, outside the cases listed for Android with the reason

#### Scenario: IO at the end of a file
- **WHEN** Ruby 1.8.7 on a phone reads an empty file and a closed pipe with `eof?` and `readpartial`
- **THEN** `eof?` is true and `readpartial` raises `EOFError` without blocking

#### Scenario: crypt
- **WHEN** `"x".crypt("ab")` is evaluated on Android
- **THEN** it raises `NotImplementedError`

#### Scenario: Too old an API level
- **WHEN** an interpreter crate is built for Android with API level 31
- **THEN** the build fails and names API level 34
