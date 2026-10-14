## ADDED Requirements

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
