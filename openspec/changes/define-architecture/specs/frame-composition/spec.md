## ADDED Requirements

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
