## ADDED Requirements

### Requirement: Recorded frame times of VX
`docs/decisions.md` SHALL record, beside the risk that asks for them, the frame times of
representative VX scenes measured before version 0.2.0: the machine and the build, the runs
and their scenes, the mean and the largest value of the stages, and what was not measured.

#### Scenario: The record is there
- **WHEN** the risks of the extension to VX in `docs/decisions.md` are read
- **THEN** the risk on frame times is followed by the measurement of 2026-10-13 with the numbers of this change's `design.md`

### Requirement: Version 0.2.0
The version of the workspace SHALL be 0.2.0, and `CHANGELOG.md` SHALL have a section
`[0.2.0] - 2026-10-13` above that of 0.1.0 which names support for RPG Maker VX.

#### Scenario: Version and lock file
- **WHEN** `cargo metadata --locked --no-deps` is run
- **THEN** it succeeds and gives the package `vkxp` the version 0.2.0

#### Scenario: Documents after V3
- **WHEN** the first paragraph of `README.md`, the first sentence of `AGENTS.md` and the openings of `docs/compatibility.md` and `docs/running.md` are read
- **THEN** each says that XP and VX are supported and that VX Ace is planned, and none calls VX experimental
