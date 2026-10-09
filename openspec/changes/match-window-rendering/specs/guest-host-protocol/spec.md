# Spec Delta

## ADDED Requirements

### Requirement: Window blend mode
The blend modes of a quad SHALL include a mode for window layers besides normal, additive and
subtractive. The protocol version SHALL be 4.

#### Scenario: Round trip
- **WHEN** a frame packet with a quad in the window blend mode is serialised and read back
- **THEN** the quad is equal to the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 4
