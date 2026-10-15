# supported-platforms Specification

## Purpose
TBD - created by archiving change plan-the-platforms-and-modernization. Update Purpose after archive.

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
