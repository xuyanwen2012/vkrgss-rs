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
A quad op SHALL draw a source rectangle of a bitmap through a fixed-point affine transform,
sampling the nearest source texel. The sampled texel SHALL first be changed by the quad's tone,
then blended with the quad's colour by that colour's alpha, both leaving the texel's own alpha
unchanged. The result SHALL then be combined with the pixel underneath at an opacity from 0 to
255 in normal, additive or subtractive mode.

#### Scenario: Zero opacity draws nothing
- **WHEN** a quad has opacity 0
- **THEN** the output is unchanged

#### Scenario: Transparent source texel draws nothing
- **WHEN** a source texel has alpha 0
- **THEN** the corresponding output pixel is unchanged in every blend mode, whatever the quad's
  tone and colour

#### Scenario: Additive and subtractive blending saturate
- **WHEN** an additive or subtractive quad would move a channel past 255 or below 0
- **THEN** the channel is clamped to 255 or 0 respectively

#### Scenario: Identity transform copies texels
- **WHEN** an opaque, fully opaque-alpha quad with a neutral tone and a colour of alpha 0 is
  drawn with an identity transform at an integer position in normal mode
- **THEN** each covered output pixel equals the corresponding source texel

#### Scenario: Neutral tone and zero-alpha colour change nothing
- **WHEN** a quad has a neutral tone and a colour whose alpha is 0, for any texel value
- **THEN** the frame equals the one produced with the texel drawn as it is

#### Scenario: Full-alpha colour replaces the texel's colour
- **WHEN** a quad's colour has alpha 255
- **THEN** every covered texel is drawn with the red, green and blue of that colour and with
  the texel's own alpha

#### Scenario: Tone is applied before colour
- **WHEN** a quad has both a non-neutral tone and a colour with an alpha between 1 and 254
- **THEN** the drawn texel equals the tone applied to the sampled texel followed by the colour
  blended over that result

#### Scenario: Tone and colour do not reach the pixels underneath
- **WHEN** a quad with a non-neutral tone and a colour is drawn at opacity 0, or a pixel inside
  its clip rectangle is not covered by its source rectangle
- **THEN** that pixel is unchanged

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

### Requirement: Blend arithmetic of the original on an opaque destination
For a quad drawn onto the frame, with `s` a source channel after tone and colour, `d` the
channel underneath and all values integers from 0 to 255, the compositor SHALL compute the
effective alpha as `a = (pixel_alpha * opacity + 127) / 255`, leave the pixel unchanged when
`a` is 0, and otherwise produce `d + (((s - d) * (a + 1)) >> 8)` in normal mode,
`min(255, d + ((s * (a + 1)) >> 8))` in additive mode and
`max(0, d - ((s * (a + 1)) >> 8))` in subtractive mode, with an arithmetic shift.

#### Scenario: Measured samples
- **WHEN** each committed sample measured on the original runtime for the three modes is evaluated
- **THEN** the result equals the measured value

#### Scenario: Opaque source
- **WHEN** pixel alpha and opacity are 255 in normal mode
- **THEN** the result equals the source channel

### Requirement: Colour and tone arithmetic of the original
The quad's colour SHALL change a channel `s` to `s + (((c - s) * (ca + 1)) >> 8)` for a colour
channel `c` with alpha `ca` above 0 and leave it unchanged for alpha 0, and the tone's red,
green and blue offsets SHALL change a channel to `s + t` clamped to 0 and 255, the tone before
the colour.

#### Scenario: Measured samples
- **WHEN** each committed colour, tone and combined sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value

### Requirement: Measured formulas are marked
The arithmetic functions that implement the two requirements above SHALL be listed with a
reverse-engineered provenance that names the measurement, and functions not yet measured SHALL
stay listed as placeholders.

#### Scenario: Provenance list
- **WHEN** the provenance of all pixel arithmetic functions is listed
- **THEN** the functions for blending onto an opaque destination, for the colour and for the tone offsets name the measurement, and at least the blend into a translucent destination is a placeholder

### Requirement: Grey tone arithmetic of the original
The grey part of a tone SHALL be applied before its red, green and blue offsets: with
`L = (77 * r + 150 * g + 29 * b) >> 8`, each channel `c` SHALL become
`c + (((L - c) * (gray + 1)) >> 8)` for a grey value above 0 and stay unchanged for 0. CPU
reference and GPU SHALL agree bit for bit, and the function SHALL be listed with a
reverse-engineered provenance.

#### Scenario: Measured samples
- **WHEN** each committed tone sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value

### Requirement: Stretch sampling of quads
A stretch quad with destination rectangle `(x0, y0, dw, dh)` and a source rectangle of size
`(w, h)` SHALL cover the pixels of the destination rectangle inside its clip, and pixel
`(x0 + i, y0 + j)` SHALL show the source texel at column `(i * ((w << 20) / dw)) >> 20` and
row `(j * ((h << 20) / dh)) >> 20`, the column counted from the right edge when the mirror
flag is set. CPU reference and GPU SHALL agree bit for bit.

#### Scenario: Measured samples
- **WHEN** each committed sprite zoom sample measured on the original runtime is drawn as a stretch quad
- **THEN** every listed screen pixel shows the measured texel

#### Scenario: Clipped
- **WHEN** a stretch quad's destination rectangle starts 100 pixels left of its clip rectangle
- **THEN** the first visible column shows the texel of `i = 100`, not of `i = 0`

#### Scenario: Unscaled
- **WHEN** the destination size equals the source size
- **THEN** every pixel shows the texel at the same offset

#### Scenario: Empty destination
- **WHEN** the destination width or height is 0 or less
- **THEN** nothing is drawn

### Requirement: Window blend arithmetic of the original
A quad in the window blend mode SHALL combine a source channel `s` with the channel underneath
`d` at effective alpha `a` as `(s * (a + 1) + d * (256 - a)) >> 8`, leaving the pixel unchanged
for `a` equal to 0. In each row, when the part of the quad that is visible inside its clip and
the frame has an odd number of pixels, the last of them SHALL use the normal mode's formula
instead. CPU reference and GPU SHALL agree bit for bit.

#### Scenario: Measured samples
- **WHEN** each committed window blend sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value

#### Scenario: Odd row
- **WHEN** a window-mode quad's visible part is 5 pixels wide
- **THEN** its first four columns use the window formula and the fifth the normal formula

### Requirement: Tilemap layer evaluation
For each pixel `(X, Y)` in its clip a tilemap layer op SHALL read the tile number of its map
layer at cell `((X + ox) div 32, (Y + oy) div 32)`, wrapped into the table's size. When the
priorities table's entry for that number (0 when it has none) equals the op's priority, the
op SHALL blend the tile's texel for that pixel onto the frame with the normal blend formula
at opacity 255; otherwise it SHALL draw nothing. CPU reference and GPU SHALL agree bit for
bit.

#### Scenario: Tileset tile
- **WHEN** a cell holds tile number 384 + 9 and the op's priority matches
- **THEN** the cell shows the 32 by 32 block at column 1, row 1 of the tileset

#### Scenario: Priority filter
- **WHEN** a cell's tile has priority 2 and the op draws priority 0
- **THEN** the cell is unchanged by this op

#### Scenario: Wrap
- **WHEN** the origin is beyond the map's size or negative
- **THEN** the map repeats in both directions

#### Scenario: Translucent texel
- **WHEN** a tile texel has alpha 142 over a frame pixel
- **THEN** the result is that of a sprite pixel with the same values

### Requirement: Tileset tiles in the tilemap op
A tile number from 384 SHALL select the tileset block at column `(number - 384) mod 8`, row
`(number - 384) div 8`, and a number below 48 SHALL draw nothing. An absent bitmap, or a texel
outside its bitmap, SHALL draw nothing.

#### Scenario: Block position
- **WHEN** tile number 384 + 17 is drawn
- **THEN** it shows the block at column 1, row 2 of the tileset

#### Scenario: Short tileset
- **WHEN** a tile number names a row beyond the tileset bitmap's height
- **THEN** the cell is unchanged

### Requirement: Autotiles in the tilemap op
A tile number from 48 to 383 SHALL select the autotile bitmap `number div 48 - 1` and the
pattern `number mod 48`, whose four 16 by 16 quarters come from the autotile sheet as the
pattern table of `core-scene` says, offset by 96 pixels times the animation step modulo the
sheet's number of frames.

#### Scenario: Every autotile pattern
- **WHEN** each of the 48 patterns is drawn from a sheet whose 16 by 16 quarters have distinct colours
- **THEN** every quarter equals the one the pattern table names

#### Scenario: Animation step
- **WHEN** a four-frame autotile sheet is drawn at animation steps 0, 1 and 5
- **THEN** the picture comes from frame 0, frame 1 and frame 1

#### Scenario: Missing autotile
- **WHEN** a cell names an autotile whose bitmap is absent
- **THEN** the cell is unchanged

### Requirement: One definition of the tile numbering
The rules that map a tile number to a sheet and a texel (the tile and quarter sizes, the
ranges of autotile and tileset numbers, the autotile quarter table, the single-tile sheet
rule) SHALL have one Rust definition, in the crate of the shared protocol types, which the
guest's extraction and the reference compositor both use. The shader's copy of the quarter
table SHALL be compared with it by a test.

#### Scenario: Shader table
- **WHEN** the shader's autotile quarter table is compared with the shared definition
- **THEN** the 192 entries are equal

#### Scenario: No guest-side composition
- **WHEN** the guest crates are built for tests
- **THEN** no code path composes a tilemap into a bitmap
