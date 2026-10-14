## REMOVED Requirements

### Requirement: State of VX Ace once its games start
**Reason**: A3 is reached; VX Ace is supported (requirement "VX Ace is supported").
**Migration**: The documents say "supported" where they said "experimental".

### Requirement: Version 0.2.0
**Reason**: Replaced by "Version 0.3.0".
**Migration**: None.

## MODIFIED Requirements

### Requirement: Order and milestones of VX Ace
`docs/decisions.md` SHALL give VX Ace the milestones A0 (the original runtime under Wine),
A1 (title screen and menu), A2 (walking on a map) and A3 (battle, saving and loading), held
to the standard of V1 to V3, and SHALL say that version 0.3.0 is released when A3 is reached.

#### Scenario: Rows
- **WHEN** the milestone table is read
- **THEN** it has the rows A0 to A3, each "Done" with the games, their DLLs, the seeds, the schedules, the compared frames and the repeated run, and A3 also with the saves written at the same frame

#### Scenario: Limits of A0
- **WHEN** the row A0 is read
- **THEN** its "Not covered" names frames inside a transition, saving, loading and reset, and the single runs of the encrypted games

## ADDED Requirements

### Requirement: VX Ace is supported
The README, the agent guide, `docs/compatibility.md`, `docs/running.md` and the usage
message SHALL say that RPG Maker XP, VX and VX Ace are supported, and none SHALL call VX Ace
experimental or planned. The README SHALL keep at most 200 lines.

#### Scenario: Documents
- **WHEN** the first paragraph and the table of runtimes of `README.md`, the first sentence of `AGENTS.md` and the openings of `docs/compatibility.md` and `docs/running.md` are read
- **THEN** each says that XP, VX and VX Ace are supported, and none calls VX Ace experimental or planned

#### Scenario: Banner
- **WHEN** the banner of the README and its alternative text are looked at
- **THEN** the card of VX Ace is labelled as supported and the text does not call it planned

#### Scenario: Length
- **WHEN** the lines of `README.md` are counted
- **THEN** there are at most 200

### Requirement: Version 0.3.0
The version of the workspace SHALL be 0.3.0, and `CHANGELOG.md` SHALL have a section
`[0.3.0] - 2026-10-14` above that of 0.2.0 which names support for RPG Maker VX Ace and
holds what the section "Unreleased" held.

#### Scenario: Version and lock file
- **WHEN** `cargo metadata --locked --no-deps` is run
- **THEN** it succeeds and gives the packages of the workspace the version 0.3.0

#### Scenario: Changelog
- **WHEN** `CHANGELOG.md` is read
- **THEN** it has no section "Unreleased" with entries, and the section of 0.3.0 starts with support for RPG Maker VX Ace
