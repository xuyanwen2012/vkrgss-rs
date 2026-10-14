## ADDED Requirements

### Requirement: Recorded decisions on sprites, snapshots and input of RGSS3
`docs/decisions.md` SHALL record, under "VX Ace", the decisions 1, 5, 8 and 10 of this
change's `design.md`: that the rules of a scene have a value for RGSS3 and what it changes;
when an input snapshot is seen and what of the original's key messages is not reproduced;
the frozen picture and what of it is decided without a measurement; and what vkxp does where
the original faults or reads outside its sine table.

#### Scenario: The decisions are there
- **WHEN** the section "VX Ace" of `docs/decisions.md` is compared with the decisions 1, 5, 8 and 10 of this change's `design.md`
- **THEN** each is stated with the date of the measurement, and the known differences name: the count of key messages a frame takes, a key pressed and released between two frames, a disposed viewport given to a move, the sprite that ends the original, and a negative wave length

#### Scenario: Nothing is claimed
- **WHEN** the README, the agent guide and `docs/compatibility.md` are read after this change
- **THEN** they do not say that a game of VX Ace runs, and `docs/compatibility.md` is unchanged
