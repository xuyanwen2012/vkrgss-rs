## MODIFIED Requirements

### Requirement: State of VX Ace once its games start
The README, the agent guide, `docs/compatibility.md`, `docs/running.md` and the usage
message SHALL say that RPG Maker VX Ace is experimental, and SHALL NOT say that it is
supported or that a milestone of it is reached. The rows A1 to A3 of the milestone table
SHALL stay "Not started". This statement SHALL replace what earlier scenarios of this
capability say of VX Ace as planned.

#### Scenario: Documents
- **WHEN** the first paragraph and the table of runtimes of `README.md`, the first sentence of `AGENTS.md` and the openings of `docs/compatibility.md` and `docs/running.md` are read
- **THEN** each says that XP and VX are supported and that VX Ace is experimental, and none says that VX Ace is planned or does not run

#### Scenario: What experimental means here
- **WHEN** the section on VX Ace of `docs/compatibility.md` is read
- **THEN** it says that the scripts of a game of VX Ace run in a guest executable of their own with the classes of RGSS3, and that no frame was compared with the original, and it does not say that a game made with the standard scripts ends before its title screen

#### Scenario: Milestones
- **WHEN** the milestone table of `docs/decisions.md` is read
- **THEN** the rows A1, A2 and A3 say "Not started" and the row A0 is unchanged

#### Scenario: README stays within its length
- **WHEN** the lines of `README.md` are counted
- **THEN** there are at most 200
