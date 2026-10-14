## ADDED Requirements

### Requirement: Known differences of the scene classes and Graphics of RGSS3
`docs/compatibility.md` SHALL have, under "RPG Maker VX Ace", a subsection on `Sprite`,
`Plane`, `Viewport` and `Graphics` that names as known differences every place where vkxp
does not do what the original of RGSS3 was measured to do, and `docs/decisions.md` SHALL
record what was decided without a measurement. The state of VX Ace in the documents SHALL
NOT change.

#### Scenario: The differences are there
- **WHEN** the subsection is compared with the decisions 4, 6, 8 and 9 and the Follow-ups of this change's `design.md`
- **THEN** it names: a disposed viewport given to `viewport=`, which ends the original and raises `RGSSError` in vkxp; the elements of a disposed viewport, drawn nowhere; `Graphics.play_movie`, which plays nothing; and `Graphics.frame_count` beyond 2^31 - 1, which does not wrap

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is read
- **THEN** it says that disposing a viewport of RGSS3 leaves its elements alive and that vkxp draws them nowhere, not measured, and how `play_movie` looks for a file, not measured

#### Scenario: VX and the tilemap of VX Ace
- **WHEN** the section "RPG Maker VX" and the subsection on the tilemap of VX Ace of `docs/compatibility.md` are read
- **THEN** the first says that vkxp disposes the elements of a disposed viewport where the original of VX keeps them, and the second no longer says that `Viewport#dispose` disposes a tilemap of VX Ace

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and the opening of `docs/compatibility.md` are compared with the start of the change
- **THEN** what they say about the state of VX Ace is unchanged, and the subsection says nothing that the code does not do
