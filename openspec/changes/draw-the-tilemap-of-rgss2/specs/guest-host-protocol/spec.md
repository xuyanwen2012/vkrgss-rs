## ADDED Requirements

### Requirement: Tilemap plane op of RGSS2
The protocol SHALL have an op for one plane of a tilemap of RGSS2 that carries which plane it
is, its clip and corner, the first cell, the animation step, the flash level, the ids of the
map, passages and flash tables, and up to nine bitmap ids. All its values SHALL be integers.

#### Scenario: Round trip
- **WHEN** a packet with a lower and an upper plane op, some bitmap slots empty, is encoded and decoded
- **THEN** the decoded packet equals the one built

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version
