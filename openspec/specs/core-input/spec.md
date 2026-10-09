# core-input Specification

## Purpose
Defines the button state that scripts query through `Input`: how it is derived from the input
snapshots the host delivers, one update at a time, so that the same snapshots always give the
same answers.

## Requirements

### Requirement: Update latches a snapshot
The input state SHALL change only when it is updated, and an update SHALL take the most recent
snapshot received since the previous update, or keep the previous button state when none was
received. Queries between two updates SHALL give the same answers however often they are asked.

#### Scenario: Snapshot arrives between updates
- **WHEN** a snapshot with C held arrives and C is queried before the next update
- **THEN** C is reported as not pressed until the update has run

#### Scenario: No new snapshot
- **WHEN** an update runs without a new snapshot since the last one
- **THEN** every button keeps its pressed state

#### Scenario: Several snapshots between updates
- **WHEN** three snapshots arrive between two updates
- **THEN** the update uses the last of them

### Requirement: Pressed and triggered
A button SHALL be pressed while the latched snapshot has it held. It SHALL be triggered only
during the update in which it changed from not held to held.

#### Scenario: Trigger lasts one update
- **WHEN** a button is held over three consecutive updates
- **THEN** it is triggered after the first of them and not after the second and third, and pressed after all three

#### Scenario: Release and press again
- **WHEN** a button is released for one update and held again
- **THEN** it is triggered again

### Requirement: Repeat
A button SHALL be repeated on the update in which it is triggered, not on the 14 updates that
follow while it stays held, and then on the 15th update after the trigger and on every 4th
update after that for as long as it stays held.

#### Scenario: Repeat pattern
- **WHEN** a button is held for 24 updates
- **THEN** it is repeated on the 1st, 16th, 20th and 24th of them and on no other

#### Scenario: Release resets the pattern
- **WHEN** a button is released after 10 updates and held again
- **THEN** the pattern starts over with a repeat on the first update

### Requirement: Direction queries
The four-way query SHALL return 2, 4, 6 or 8 for down, left, right or up, or 0 for none. Opposite
directions held together SHALL cancel; of two perpendicular directions the one pressed more
recently SHALL be returned. The eight-way query SHALL return 1, 3, 7 or 9 for down-left,
down-right, up-left or up-right when two perpendicular directions are held, and otherwise the
value of the four-way query.

#### Scenario: Single direction
- **WHEN** only LEFT is held
- **THEN** both queries return 4

#### Scenario: Opposites cancel
- **WHEN** LEFT and RIGHT are held
- **THEN** both queries return 0

#### Scenario: Diagonal
- **WHEN** DOWN is held and RIGHT is pressed afterwards
- **THEN** the four-way query returns 6 and the eight-way query returns 3
