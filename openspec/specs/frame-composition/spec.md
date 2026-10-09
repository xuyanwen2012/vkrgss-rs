# frame-composition Specification

## Purpose
Defines how a compositor backend turns an ordered op list into pixels, so that the CPU reference and every GPU backend produce byte-identical frames.

## Requirements

### Requirement: Ordered per-pixel evaluation
A compositor SHALL produce each output pixel by applying, in op-list order, every op whose
extent and clip rectangle contain that pixel, starting from the result of the previous op.

#### Scenario: Later op draws over earlier op
- **WHEN** two opaque quads overlap and the second follows the first in the op list
- **THEN** the overlap shows the second quad's pixels

#### Scenario: Clip rectangle limits an op
- **WHEN** a quad extends beyond its clip rectangle
- **THEN** pixels outside the clip rectangle are unchanged by that op

### Requirement: Clear op
A clear op SHALL set every pixel in its clip rectangle to its colour.

#### Scenario: Full-screen clear
- **WHEN** a frame consists of one clear op covering the logical screen
- **THEN** every output pixel equals the clear colour

### Requirement: Quad op
A quad op SHALL draw a source rectangle of a bitmap through a fixed-point affine transform with
an opacity from 0 to 255 and a blend mode of normal, additive or subtractive, sampling the
nearest source texel.

#### Scenario: Zero opacity draws nothing
- **WHEN** a quad has opacity 0
- **THEN** the output is unchanged

#### Scenario: Transparent source texel draws nothing
- **WHEN** a source texel has alpha 0
- **THEN** the corresponding output pixel is unchanged in every blend mode

#### Scenario: Additive and subtractive blending saturate
- **WHEN** an additive or subtractive quad would move a channel past 255 or below 0
- **THEN** the channel is clamped to 255 or 0 respectively

#### Scenario: Identity transform copies texels
- **WHEN** an opaque, fully opaque-alpha quad is drawn with an identity transform at an integer
  position in normal mode
- **THEN** each covered output pixel equals the corresponding source texel

### Requirement: Rectangle effect op
A rectangle effect op SHALL modify the pixels already present inside its rectangle by a tone
(signed per-channel offsets and a grey amount) and a colour blend, without reading any bitmap.

#### Scenario: Neutral effect is a no-op
- **WHEN** a rectangle effect has zero tone and a colour with zero alpha
- **THEN** the output is unchanged

#### Scenario: Effect applies to what is underneath
- **WHEN** a rectangle effect follows ops that drew inside its rectangle
- **THEN** it changes those pixels and leaves pixels outside the rectangle unchanged

### Requirement: Backends agree bit for bit
Every compositor backend SHALL produce byte-identical output for the same sequence of messages.

#### Scenario: Deterministic reference output
- **WHEN** the CPU reference compositor processes the same messages twice
- **THEN** the two output images are byte-identical

### Requirement: Arithmetic provenance
Each pixel arithmetic function SHALL declare whether its formula is a textbook placeholder or
derived from a named reverse-engineering source, and that declaration SHALL be queryable.

#### Scenario: Listing placeholders
- **WHEN** the provenance of all pixel arithmetic functions is listed
- **THEN** every function appears with either a placeholder marker or a named source

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
