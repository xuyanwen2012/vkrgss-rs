# Spec Delta

## ADDED Requirements

### Requirement: Quad sampling variants
A quad SHALL state how it is sampled as one of two plain-data variants: an affine transform, as
before, or a stretch, which names an integer destination rectangle in logical pixels and a
mirror flag. The protocol version SHALL be 3.

#### Scenario: Round trip
- **WHEN** a frame packet with one affine quad and one stretch quad is serialised and read back
- **THEN** both quads are equal to the originals

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 3
