## ADDED Requirements

### Requirement: Known differences of Input of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Input` that
says what a game of VX Ace gives `Input` and when it sees a key, that its keys are those of
VX, and names as known differences every place where vkxp does not do what the original of
RGSS3 was measured to do. The state of VX Ace in the documents SHALL NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with decision 5 of the archived `draw-sprites-and-snapshots-as-rgss3` and the Follow-ups of this change's `design.md`
- **THEN** it names: the count of key messages a frame of the original takes, a key pressed and released between two frames, the key the original takes during `Kernel#sleep`, and the key functions of `Win32API`, which follow the rules of VX

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do
