## ADDED Requirements

### Requirement: Rule set of the tilemap plane op
The tilemap plane op SHALL carry a rule set, that of RGSS2 or that of RGSS3, as an integer
value like its other fields. With the rule set of RGSS3 the table id it carries for the
passages SHALL name the flags table, and the map table SHALL be sent with all its layers.
The protocol version SHALL be one above the version before this change.

#### Scenario: Round trip
- **WHEN** a packet with a lower plane op of the rule set of RGSS2 and an upper plane op of the rule set of RGSS3, some bitmap slots empty, is encoded and decoded
- **THEN** the decoded packet equals the one built, rule sets included

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version, which states both versions

#### Scenario: Four layers
- **WHEN** a table of 4 by 3 by 4 elements is sent as the map of a plane op of the rule set of RGSS3 and decoded
- **THEN** the decoded table has its 48 elements in the order sent

#### Scenario: Documents name the version
- **WHEN** `README.md` and `docs/architecture.md` are read where they state the protocol version
- **THEN** both give the value of `PROTOCOL_VERSION`, and `docs/architecture.md` says that this version added the rule set of the tilemap plane op
