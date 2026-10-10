# Spec Delta

## REMOVED Requirements

### Requirement: Reserved ops are rejected explicitly
**Reason**: The only reserved op kind, `Plane`, is removed; every op kind the protocol can
express is implemented by both backends.
**Migration**: None for games. A recording or message stream of protocol version 6 is
rejected by the version check and has to be made again.

## ADDED Requirements

### Requirement: Every op kind is implemented
The protocol SHALL contain only op kinds that both backends evaluate. The protocol version
SHALL be 7.

#### Scenario: Version 6 stream
- **WHEN** a decoder reads a stream that declares protocol version 6
- **THEN** it returns the unsupported-version error naming 6 and 7

#### Scenario: Round trip of each kind
- **WHEN** one op of each kind is encoded and decoded
- **THEN** each comes back equal, and both backends accept a packet that holds them
