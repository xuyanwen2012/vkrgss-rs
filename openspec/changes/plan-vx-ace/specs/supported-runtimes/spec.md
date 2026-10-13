## ADDED Requirements

### Requirement: Order and milestones of VX Ace
`docs/decisions.md` SHALL give VX Ace the milestones A0 (the original runtime under Wine),
A1 (title screen and menu), A2 (walking on a map) and A3 (battle, saving and loading), held
to the standard of V1 to V3, and SHALL say that version 0.3.0 is released when A3 is reached.

#### Scenario: Rows
- **WHEN** the milestone table is read
- **THEN** it has the rows A0 to A3, A0 "Done" with the games, their DLLs, the seeds, the schedule and the repeated run, and A1 to A3 "Not started"

#### Scenario: Limits of A0
- **WHEN** the row A0 is read
- **THEN** its "Not covered" names frames inside a transition, saving, loading and reset, and the single runs of the encrypted games

### Requirement: Recorded decisions on VX Ace
`docs/decisions.md` SHALL record the decisions of this change on VX Ace: the guest
executable and how it is chosen and checked, `--guest thread`, the vendored interpreter and
its sys crate, the one source of the binding, encodings, the hash seed, script threads, the
word size, the order of work, the original side without device nodes, and the process.

#### Scenario: Each decision is there
- **WHEN** the section "VX Ace" and the section "Process" of `docs/decisions.md` are compared with the section "Decisions" of this change's `design.md`
- **THEN** every one of the twelve decisions is stated

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they still say that VX Ace is planned
