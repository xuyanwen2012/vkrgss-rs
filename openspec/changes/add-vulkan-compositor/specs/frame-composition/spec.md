# Spec Delta

## ADDED Requirements

### Requirement: Initial frame state
A compositor SHALL start every frame with every pixel opaque black (red 0, green 0, blue 0,
alpha 255) before applying the first op. A frame SHALL NOT depend on the pixels of any earlier
frame.

#### Scenario: Empty op list
- **WHEN** a frame packet has no ops
- **THEN** every output pixel is opaque black

#### Scenario: Previous frame does not show through
- **WHEN** a packet that fills the screen is followed by a packet with no ops
- **THEN** the second frame is opaque black everywhere

### Requirement: Quad sampling position
A quad SHALL cover a screen pixel exactly when its transform maps the centre of that pixel to a
position inside the source rectangle. Each sum of products in the transform SHALL be computed
exactly and rounded towards negative infinity to 16 fractional bits before the translation is
added, and the texel SHALL be the one whose integer coordinates are the floor of that position.

#### Scenario: Pixel centre outside the source rectangle
- **WHEN** a pixel centre maps to a position with a negative coordinate or one at or beyond the
  source rectangle's width or height
- **THEN** the quad leaves that pixel unchanged

#### Scenario: Extreme coefficients
- **WHEN** a transform uses the largest and smallest representable fixed-point coefficients
- **THEN** every backend selects the same texel for every pixel as exact integer arithmetic does

### Requirement: Failed packets produce no frame and keep bitmap state
When a frame packet is rejected, a compositor SHALL produce no frame for it. Bitmap deltas that
precede the first invalid delta SHALL stay applied, and an op error SHALL leave every delta of
the packet applied, so that later packets are interpreted against the same bitmap state by every
backend.

#### Scenario: Op error after valid deltas
- **WHEN** a packet creates a bitmap and then contains an invalid op
- **THEN** no frame is produced and a later packet can draw the bitmap

#### Scenario: Delta error part way through
- **WHEN** the second of three bitmap deltas in a packet is invalid
- **THEN** no frame is produced, the first delta is applied and the third is not
