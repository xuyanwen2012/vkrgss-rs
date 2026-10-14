## ADDED Requirements

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
- **THEN** it says that the scripts of a game of VX Ace run in a guest executable of their own, that the classes are those of RGSS2 still, that a game made with the standard scripts therefore ends with an error before its title screen, and that no frame was compared with the original

#### Scenario: Milestones
- **WHEN** the milestone table of `docs/decisions.md` is read
- **THEN** the rows A1, A2 and A3 say "Not started" and the row A0 is unchanged

#### Scenario: README stays within its length
- **WHEN** the lines of `README.md` are counted
- **THEN** there are at most 200

### Requirement: Recorded decisions on the guest executable and the start of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 5 to 16 of this change's
`design.md`: where the guest executable is looked for and that nothing overrides it, how
tests reach it, the form of the guest and the experimental state, the preparation, script
loading, data files, reset, endings and the report with what of the original is not
followed, the message boxes and console output, states with script threads, and the
release archive.

#### Scenario: Each decision is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 5 to 16 of this change's `design.md`
- **THEN** every one of them is stated, and each difference from the original that the design names as not followed is named as a known difference

#### Scenario: The changes that follow
- **WHEN** the section "VX Ace" is read
- **THEN** it says that the class surface of RGSS3 is bound by the changes that follow and names what a title screen of the standard scripts calls of it
