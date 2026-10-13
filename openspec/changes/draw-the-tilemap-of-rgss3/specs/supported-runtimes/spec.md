## ADDED Requirements

### Requirement: Recorded decisions on the tilemap of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", what vkxp draws where the tilemap of the
original runtime of RGSS3 reads outside its arrays: nothing for the tile numbers 1024 to
1535 and for negative numbers, and an empty layer for each layer a map lacks; and SHALL name
as known differences the negative numbers that draw a block on the original and the maps of
fewer than three layers.

#### Scenario: The decision is there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with decision 4 of this change's `design.md`
- **THEN** the three choices and the two known differences are stated, with the date of the measurement

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` is unchanged
