## ADDED Requirements

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
V3 of VX with their status, and SHALL state that no change implementing VX is drafted before
V0 has passed.

#### Scenario: Milestone table
- **WHEN** the section "Milestones" is read
- **THEN** it has the rows V0 to V3, each with its outcome and the status "Not started", below the rows of XP, which are unchanged

### Requirement: Recorded decisions on several runtimes
`docs/decisions.md` SHALL record the decisions of this change on the structure of the
program, the corpus, the experimental state before V3, the direction for VX Ace and the
process.

#### Scenario: Each decision is there
- **WHEN** `docs/decisions.md` is compared with the section "Decisions" of this change's `design.md`
- **THEN** every decision of that section is stated in the document

### Requirement: Documents say what is supported
The README, the agent guide and `docs/compatibility.md` SHALL say that RPG Maker XP is
supported and that VX and VX Ace are planned, and SHALL NOT say that a VX or VX Ace game runs
before a change makes that true.

#### Scenario: README
- **WHEN** the first paragraph of `README.md` is read
- **THEN** it names XP as supported and VX and VX Ace as planned
