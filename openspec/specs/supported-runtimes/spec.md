# supported-runtimes Specification

## Purpose
Defines which RPG Maker runtimes the project covers, to what standard and in what order, and
how the documents state what works at a given time.

## Requirements

### Requirement: Runtimes in scope
`docs/decisions.md` SHALL name RGSS1, RGSS2 and RGSS3 as the runtimes in scope and RPG Maker
MV, MZ, 2000 and 2003 as out of scope, and SHALL state that VX and VX Ace are held to the
fidelity standard of XP.

#### Scenario: Scope
- **WHEN** the section "Goal and scope" of `docs/decisions.md` is read
- **THEN** it names the three runtimes as in scope and no longer says that RGSS2 and RGSS3 are out of scope

#### Scenario: Standard
- **WHEN** the same section is read
- **THEN** it says that frames, saves and determinism of VX and VX Ace are compared with the original as those of XP are

### Requirement: Order and milestones of VX
`docs/decisions.md` SHALL state that VX comes before VX Ace, SHALL list the milestones V0 to
V3 of VX with a status that matches the evidence recorded for each, and SHALL state that no
change implementing VX in the guest is drafted before V0 has passed.

#### Scenario: Milestone table
- **WHEN** the section "Milestones" is read
- **THEN** it has the rows V0 to V3 with outcome, status, evidence and what is not covered, below the rows of XP, which are unchanged

#### Scenario: Status follows evidence
- **WHEN** a row says a milestone is done
- **THEN** its evidence names the games, their DLLs, the input schedules and the compared frames; a milestone without such a record says "Not started" or "In progress"

### Requirement: Recorded decisions on several runtimes
`docs/decisions.md` SHALL record the decisions of this change on the structure of the
program, the corpus, the experimental state before V3, the direction for VX Ace and the
process.

#### Scenario: Each decision is there
- **WHEN** `docs/decisions.md` is compared with the section "Decisions" of this change's `design.md`
- **THEN** every decision of that section is stated in the document

### Requirement: Documents say what is supported
The README, the agent guide and `docs/compatibility.md` SHALL state for each runtime in
scope whether it is supported, experimental or planned, and that statement SHALL match the
milestones recorded in `docs/decisions.md`: a runtime is supported when its milestones are
done, experimental while its games start but its milestones are open, planned before.

#### Scenario: README
- **WHEN** the first paragraph of `README.md` is read
- **THEN** it gives the state of XP, VX and VX Ace, each as the milestone table has it

#### Scenario: Agent guide and compatibility
- **WHEN** the first sentence of `AGENTS.md` and the opening of `docs/compatibility.md` are read
- **THEN** neither says more about VX or VX Ace than the milestone table supports

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
