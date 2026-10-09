# Spec Delta

## ADDED Requirements

### Requirement: Raw key and pointer state
An input snapshot SHALL carry, besides the buttons, one bit for each of the 256 Windows
virtual-key codes and an optional pointer position in logical frame pixels, all plain data.
The empty snapshot SHALL have no key held and no pointer. The protocol version SHALL be 5.

#### Scenario: Round trip
- **WHEN** a snapshot with the virtual keys 0x01 and 0x41 held and the pointer at (100, 50) is serialised and read back
- **THEN** it equals the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 5
