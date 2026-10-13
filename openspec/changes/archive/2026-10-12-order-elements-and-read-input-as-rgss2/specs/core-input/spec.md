## ADDED Requirements

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
