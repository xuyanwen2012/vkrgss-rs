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
At most one button SHALL repeat at a time: the one triggered most recently, and of several
triggered in the same update the one with the highest button number. It SHALL be repeated on
the update in which it is triggered, and then on the 15th update after that and on every 4th
update after that for as long as it stays held. When it is released no button repeats until
one is triggered.

#### Scenario: Repeat pattern
- **WHEN** a button is held alone for 24 updates
- **THEN** it is repeated on the 1st, 16th, 20th and 24th of them and on no other

#### Scenario: Release resets the pattern
- **WHEN** a button is released after 10 updates and held again
- **THEN** the pattern starts over with a repeat on the first update

#### Scenario: A second button takes the repeat
- **WHEN** DOWN is held from update 1 and LEFT in addition from update 8, for 31 updates
- **THEN** DOWN is repeated on update 1 only, and LEFT on updates 8, 23, 27 and 31

#### Scenario: The first button does not get it back
- **WHEN** DOWN is held from update 1 to 40 and LEFT only during update 2
- **THEN** DOWN is repeated on update 1, LEFT on update 2, and nothing on updates 3 to 40

#### Scenario: Same update
- **WHEN** B and C are first held in the same update
- **THEN** both are triggered and only C is repeated

### Requirement: Direction queries
The four-way query SHALL return 2, 4, 6 or 8 for down, left, right or up, or 0 for none, and
the eight-way query 1, 3, 7 or 9 in addition for the diagonals. Opposite directions held
together SHALL cancel. When a horizontal and a vertical direction remain, the four-way query
SHALL return the direction of the axis that was not the last to be active alone, and the
horizontal one when neither axis has been active alone yet.

#### Scenario: Single direction
- **WHEN** only LEFT is held
- **THEN** both queries return 4

#### Scenario: Opposites cancel
- **WHEN** LEFT and RIGHT are held
- **THEN** both queries return 0

#### Scenario: Diagonal
- **WHEN** DOWN is held and RIGHT is pressed afterwards
- **THEN** the four-way query returns 6 and the eight-way query returns 3

#### Scenario: Pressed together
- **WHEN** LEFT was the last direction held alone, everything is released, and LEFT and UP are then held from the same update
- **THEN** the four-way query returns 8 and the eight-way query returns 7

#### Scenario: Cancelled axis does not count
- **WHEN** DOWN is held, then UP in addition, then LEFT in addition, and DOWN is then released
- **THEN** the four-way query returns 2, 0, 4 and 8 at the four updates

### Requirement: Repeat timing of RGSS2
With the rules of RGSS2 the repeating button SHALL be repeated on the update in which it is
triggered, then on the 23rd update after that and on every 6th update after that while it
stays held. Everything else of "Repeat", "Pressed and triggered" and "Direction queries" SHALL
hold unchanged, and the rules of RGSS1 SHALL keep their timing.

#### Scenario: Repeat pattern, measured on the original
- **WHEN** a button is held alone for 36 updates under the rules of RGSS2
- **THEN** it is repeated on the 1st, 24th, 30th and 36th of them and on no other

#### Scenario: Sequences of the measurement
- **WHEN** each committed sequence of held buttons measured on the original runtime of VX is replayed
- **THEN** `press?`, `trigger?`, `repeat?`, `dir4` and `dir8` equal the measured values at every update

#### Scenario: XP
- **WHEN** a button is held alone for 24 updates under the rules of RGSS1
- **THEN** it is repeated on the 1st, 16th, 20th and 24th

### Requirement: Input of RGSS3
An input state SHALL be creatable with the rules of RGSS3. With them the repeat timing SHALL
be that of "Repeat timing of RGSS2", and "Repeat", "Pressed and triggered" and "Direction
queries" SHALL hold as for RGSS2, applied to the snapshot an update latches by "Snapshots
are seen at shown frames in RGSS3".

#### Scenario: Repeat pattern, measured on the original
- **WHEN** a button is held alone for 36 updates under the rules of RGSS3, a frame being shown before each update
- **THEN** it is repeated on the 1st, 24th, 30th and 36th of them and on no other

#### Scenario: Sequences of the measurement
- **WHEN** each of the 12084 updates measured on the original runtime of VX Ace, all committed, is replayed with the held buttons the original reported, a frame being shown before each update
- **THEN** `press?`, `trigger?`, `repeat?`, `dir4` and `dir8` equal the measured values at every update

#### Scenario: Not the timing of RGSS1
- **WHEN** the same committed updates are replayed under the rules of RGSS1
- **THEN** at least one value of `repeat?` differs from the measured one

#### Scenario: First diagonal of a new state, measured on the original
- **WHEN** LEFT and UP are held from the same update as the very first input of a new state of RGSS3, a frame being shown before that update
- **THEN** the four-way query returns 4 and the eight-way query 7

### Requirement: Snapshots are seen at shown frames in RGSS3
The input state SHALL offer a call that marks a shown frame. With the rules of RGSS3 a
received snapshot SHALL become visible at the next such call, and an update SHALL latch the
most recent visible snapshot that no update has latched yet, or keep the button state when
there is none. With the rules of RGSS1 and RGSS2 the call SHALL change nothing and "Update
latches a snapshot" SHALL hold as before.

#### Scenario: An update alone sees nothing new
- **WHEN** under the rules of RGSS3 a snapshot with C held arrives and three updates run without a shown frame
- **THEN** C is not pressed after any of them

#### Scenario: Seen after a shown frame
- **WHEN** a frame is then marked as shown and one update runs
- **THEN** C is pressed and triggered

#### Scenario: Arrived after the frame
- **WHEN** under the rules of RGSS3 a frame is marked as shown, then a snapshot with C held arrives, then an update runs, then a frame is marked as shown and an update runs
- **THEN** C is not pressed after the first update and is pressed and triggered after the second

#### Scenario: Several snapshots before one frame
- **WHEN** under the rules of RGSS3 a snapshot with C held and then one with nothing held arrive, a frame is marked as shown and an update runs
- **THEN** C is neither pressed nor triggered

#### Scenario: Several frames before one update
- **WHEN** under the rules of RGSS3 a snapshot with C held arrives and twelve frames are marked as shown before one update runs
- **THEN** C is pressed and triggered after that update

#### Scenario: RGSS1 and RGSS2
- **WHEN** under the rules of RGSS1, and of RGSS2, a snapshot with C held arrives and one update runs, with and without a frame marked as shown before it
- **THEN** C is pressed and triggered in all four cases
