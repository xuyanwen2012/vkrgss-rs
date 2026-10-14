## ADDED Requirements

### Requirement: Rule of a freeze message
A freeze message SHALL say by which rule the frozen picture is kept: as the frame at its own
size, which is what every freeze meant before this change, or in the buffer of RGSS3. The
rule SHALL be an integer value like the rule of a transition message. The protocol version
SHALL be one above the version before this change.

#### Scenario: Rule travels with the message
- **WHEN** two freeze messages with the two rules are encoded and decoded
- **THEN** each decoded message reports the rule it was built with

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version, which states both versions

#### Scenario: Documents name the version
- **WHEN** `README.md` and `docs/architecture.md` are read where they state the protocol version
- **THEN** both give the value of `PROTOCOL_VERSION`, and `docs/architecture.md` says that this version added the rule of the freeze message
