# Spec Delta

## MODIFIED Requirements

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
