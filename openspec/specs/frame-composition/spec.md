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
stay listed as placeholders. The additive and the subtractive blend SHALL each have one
formula, the measured one, for every destination, and SHALL keep the destination's alpha.

#### Scenario: Provenance list
- **WHEN** the provenance of all pixel arithmetic functions is listed
- **THEN** the functions for blending, for the colour and for the tone offsets name the measurement, and no blend function is a placeholder

#### Scenario: Destination with alpha
- **WHEN** the colour (200, 100, 255) with alpha 128 is added with opacity 255 to the destination (10, 20, 30) with alpha 100
- **THEN** the result is (110, 70, 158) with alpha 100

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

### Requirement: Brightness of the frame
After the last op of a packet, a compositor SHALL replace each colour channel `v` of every
pixel of the frame by `(v * b) >> 8`, where `b` is the brightness of the packet, unless `b` is
255, in which case the frame SHALL stay as composed. The alpha of the frame SHALL NOT change.

#### Scenario: Measured on the original runtime of VX
- **WHEN** a frame whose pixels have the level 200 in every colour channel is composed at brightness 100
- **THEN** every colour channel of the frame is 78

#### Scenario: Identity
- **WHEN** any packet is composed at brightness 255
- **THEN** the frame equals the frame of the same packet before this change, byte for byte

#### Scenario: Black
- **WHEN** a frame of any content is composed at brightness 0
- **THEN** every colour channel of the frame is 0

#### Scenario: Every level
- **WHEN** frames that hold every channel value from 0 to 255 are composed at each brightness from 0 to 254
- **THEN** every channel equals `(v * b) >> 8`, in the reference and in the Vulkan backend alike

### Requirement: Tiles of a tilemap plane of RGSS2
A tilemap plane op of RGSS2 SHALL draw, for each cell of its 21 by 16 cells and each of the
three layers in order, the tile the map names by the numbering, sheets, autotile shapes and
animation of rules 1 to 3 of this change's design. A number from 1024 to 1535, a negative
number, an absent bitmap and a source pixel outside its bitmap SHALL draw nothing. CPU
reference and GPU SHALL agree bit for bit.

#### Scenario: Every number and shape
- **WHEN** each committed case of the measurement that shows single tile numbers of every range and every shape of every autotile kind is composed from sheets with distinct pixels
- **THEN** every pixel equals the expected picture of the case

#### Scenario: Animation, measured on the original
- **WHEN** a map of A1 tiles is composed at the animation steps 0 to 5
- **THEN** floors of the animated kinds come from the block moved by 0, 64, 128, 64, 0 and 64 pixels, kinds 2 and 3 do not move, and waterfalls come from the block moved down by 0, 32, 64, 0, 32 and 64

#### Scenario: Undefined numbers
- **WHEN** a map holds the numbers 1024, 1535 and -256
- **THEN** their cells are unchanged

### Requirement: Planes, tables and shadows of RGSS2
The lower plane SHALL draw the tiles whose entry of the passages table lacks the bit 0x10, the
upper plane the tiles of layer 2 that have it, and no plane a tile of layer 0 or 1 that has
it. The lower plane SHALL draw the table tiles, the edge under a table and the shadows by
rule 4 of this change's design. Nothing SHALL be drawn outside the 672 by 512 pixels of a
plane.

#### Scenario: Cases of the measurement
- **WHEN** each committed case of planes, tables and shadows is composed
- **THEN** every pixel equals the expected picture of the case

#### Scenario: Shadow, measured on the original
- **WHEN** a cell of an A2 floor that is not a table has A4 tiles in layer 0 at its left and upper left neighbours, over a frame level of 200 in that cell
- **THEN** the left 16 by 32 pixels of the cell show the tile darkened by `(v * 127) >> 8`

#### Scenario: Star tile in a lower layer, measured on the original
- **WHEN** a tile with the bit 0x10 lies in layer 0
- **THEN** neither plane draws it

### Requirement: Blend and flash of a tilemap plane of RGSS2
A tile pixel SHALL be blended as `(s * (a + 1) + d * (256 - a)) >> 8`, the last pixel of a
clipped row of odd length by the formula of a sprite. Each plane SHALL then add to every
channel of a cell whose flash entry is above 0 the entry's nibble for that channel times the
op's flash level, limited to 255.

#### Scenario: Translucent tiles, measured on the original
- **WHEN** the committed case of translucent tiles over a coloured frame is composed
- **THEN** every pixel equals the expected picture

#### Scenario: Flash twice
- **WHEN** a cell of level 100 has the flash entry 0x111 and the level is 4, with both planes drawn
- **THEN** the cell shows 108

### Requirement: Whole tilemaps of RGSS2 as measured
The CPU reference and the Vulkan backend SHALL draw scenes with tilemaps of RGSS2 as the model
of the measurement does.

#### Scenario: Random scenes
- **WHEN** forty committed frames of the measurement's random generator are composed by both backends
- **THEN** every frame equals its expected picture, byte for byte

### Requirement: Rule set of a tilemap plane op
A tilemap plane op SHALL be evaluated by the rule set it names. With that of RGSS2 nothing
changes. With that of RGSS3 the numbering, sheets, autotile shapes, animation, plane size,
wrapping, blend, flash and the drawing of a table tile SHALL be those of RGSS2, and tables,
the edge under a table, shadows and star tiles SHALL follow the requirements of RGSS3 below.

#### Scenario: RGSS2 unchanged
- **WHEN** the tests of `crates/vkrgss-compose-ref/tests/tilemap_rgss2.rs` run with their ops naming the rule set of RGSS2
- **THEN** they pass, and `tests/data/tilemap-rgss2-digests.txt` of that crate is byte for byte what it was before this change

#### Scenario: Common numbering, measured on the original
- **WHEN** in the small map of this change's design layer 0 of cell (1, 1) is 1545 and layer 2 of cell (2, 1) is 37
- **THEN** pixel (35, 40) shows A5(35, 40) and pixel (67, 40) shows B(163, 136)

#### Scenario: Common animation, measured on the original
- **WHEN** layer 0 of cell (1, 1) of the small map is 2048 and the op is composed at the animation steps 0, 1, 2 and 3
- **THEN** pixel (35, 40) shows A1(35, 72), A1(99, 72), A1(163, 72) and A1(99, 72)

#### Scenario: Common blend, measured on the original
- **WHEN** layer 0 of cell (1, 1) of the small map is 1545, the sheets are those of the generated set whose every alpha is 128, and the ground has level 200 in every channel
- **THEN** pixel (35, 40) shows (140, 112, 182), each channel being `(s * 129 + 200 * 128) >> 8` of the texel (81, 24, 163)

#### Scenario: Common flash, measured on the original
- **WHEN** the small map is empty over a ground of level 100, the flash table of 4 by 3 has the entry 0x111 at (0, 0), the flash level is 4, and the lower and the upper plane are both composed
- **THEN** pixel (4, 4) shows 108 in every channel and pixel (40, 4) shows 100

### Requirement: Tables of RGSS3
In the rule set of RGSS3 a tile SHALL be a table when its number is from 2816 to 4351 and
its entry of the flags table has the bit 0x80, in any layer and on either plane; the column
of its autotile SHALL NOT matter and the bit SHALL do nothing on other numbers. The entry of
a number SHALL be the element at that number, and 0 for a negative number, for a number not
below the x size of the flags table, and without a flags table.

#### Scenario: A table by its flag, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 2844 and its flag is 0x80
- **THEN** the pixels (35, 4), (35, 16), (52, 23), (35, 24), (52, 31), (35, 32) and (52, 39) show A2(35, 68), A2(35, 48), A2(20, 55), A2(35, 80), A2(20, 87), A2(35, 88) and A2(20, 95), and pixel (35, 40) is the ground

#### Scenario: Without the flag, measured on the original
- **WHEN** the same map is composed with the flag 0, and again without a flags table, and again with a flags table of 100 elements that are all 0x80
- **THEN** each time the pixels (35, 16) and (35, 24) show A2(35, 80) and A2(35, 88) and pixel (35, 32) is the ground

#### Scenario: Column 7 is no table, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 3180 and its flag is 0
- **THEN** the pixels (35, 16) and (35, 24) show A2(483, 80) and A2(483, 88) and pixel (35, 32) is the ground

#### Scenario: The flag on another sheet, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 4380 and its flag is 0x80
- **THEN** the pixels (35, 16) and (35, 24) show A3(3, 0) and A3(3, 8) and pixel (35, 32) is the ground

#### Scenario: A table in layer 2, measured on the original
- **WHEN** layer 2 of cell (1, 0) of the small map is 2844 and its flag is 0x80
- **THEN** the pixels (35, 16), (35, 24) and (35, 32) show A2(35, 48), A2(35, 80) and A2(35, 88)

#### Scenario: Band of a table, measured on the original
- **WHEN** layer 0 of cell (1, 0) of the small map is 2844 with the flag 0x80 and the corner of the op is (0, -3)
- **THEN** the pixels (35, 28), (35, 29) and (35, 31) show A2(35, 87), A2(35, 88) and A2(35, 90), and pixel (35, 32) is the ground

### Requirement: Edge under a table of RGSS3
In the rule set of RGSS3 the lower plane SHALL draw into rows 0 to 7 of a cell the rows 88
to 95 of the two lower pieces of the table in layer 1 of the map cell above, when that
table's shape has the lower edge, the cell's layer-1 tile is no table and its layer-0 tile
is not from 4352 to 8191. It SHALL be drawn after the cell's layer 1 and shadow and before
its layer 2. The cell above SHALL NOT be looked up across the edge of the map.

#### Scenario: The edge, measured on the original
- **WHEN** in the small map layer 1 of cell (1, 0) is 2844 with the flag 0x80 and layer 0 of cell (1, 1) is 1545
- **THEN** the pixels (35, 32) and (52, 39) show A2(35, 88) and A2(20, 95), and pixel (35, 40) shows A5(35, 40)

#### Scenario: Tables of layers 0 and 2 give none, measured on the original
- **WHEN** the table 2844 lies in layer 0 of cell (1, 0) instead, and again in layer 2
- **THEN** each time the pixels (35, 32) and (52, 39) show A5(35, 32) and A5(52, 39)

#### Scenario: A shape without the lower edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2816 with the flag 0x80 and layer 0 of cell (1, 1) is 1545
- **THEN** pixel (35, 32) shows A5(35, 32)

#### Scenario: The tile of layer 1 of the cell, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545, and layer 1 of cell (1, 1) is first 2844 and then 2864 with the flag 0
- **THEN** pixel (35, 32) shows A2(35, 64) the first time, and A2(35, 88) the second time with A2(99, 72) at pixel (35, 40)

#### Scenario: The tile of layer 0 of the cell, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80 and layer 0 of cell (1, 1) is first 4352 and then 2892 with the flag 0x80
- **THEN** pixel (35, 32) shows A3(35, 32) the first time and A2(35, 88) the second time

#### Scenario: Layer 2 covers the edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545 and layer 2 of cell (1, 1) is 37
- **THEN** pixel (35, 32) shows B(163, 128)

#### Scenario: An empty cell takes the edge, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80 and cell (1, 1) is empty
- **THEN** the pixels (35, 32) and (52, 39) show A2(35, 88) and A2(20, 95) and pixel (35, 40) is the ground

#### Scenario: No look-up across the edge of the map, measured on the original
- **WHEN** layer 1 of cell (1, 2) is 2844 with the flag 0x80 and layer 0 of cell (1, 0) is 1545, so that the plane shows cell (1, 0) again under cell (1, 2)
- **THEN** the pixels (35, 0) and (35, 96) show A5(35, 32) and pixel (35, 95) shows A2(35, 87)

#### Scenario: A star table in layer 1, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x90 and layer 0 of cell (1, 1) is 1545
- **THEN** the lower plane shows A2(35, 68) at pixel (35, 4) and A2(35, 88) at pixel (35, 32)

### Requirement: Shadow mask of RGSS3
In the rule set of RGSS3 no shadow SHALL follow from walls. For each of the bits 0 to 3 set
in the cell's element of layer 3 the lower plane SHALL darken one quarter of the cell, in
this order upper left, upper right, lower left, lower right: each channel `v` becomes
`(v * 127) >> 8`. It SHALL do so after the cell's layer 1 and before the edge under a table
and layer 2. Higher bits SHALL have no effect, and the upper plane SHALL draw no shadow.

#### Scenario: Quarters, measured on the original
- **WHEN** the small map is empty over a ground of level 200 and layer 3 is -1 in cell (1, 1), 1 in cell (2, 1) and 8 in cell (3, 1)
- **THEN** every pixel of cell (1, 1) shows 99; of cell (2, 1) pixel (64, 32) shows 99 and the pixels (80, 32) and (64, 48) show 200; of cell (3, 1) pixel (127, 63) shows 99 and the pixels (112, 47) and (111, 48) show 200; pixel (35, 31) shows 200

#### Scenario: Low bits only, measured on the original
- **WHEN** layer 0 of cell (1, 1) is 1545 and layer 3 of the cell is 5, then 0x3f0a, then 16
- **THEN** with 5 the pixels (35, 40) and (35, 56) show (40, 11, 80) and (40, 11, 112) and the pixels (52, 40) and (52, 56) show A5(52, 40) and A5(52, 56); with 0x3f0a the pixels (52, 40) and (52, 56) show (40, 79, 80) and (40, 79, 112) and the pixels (35, 40) and (35, 56) show A5(35, 40) and A5(35, 56); with 16 nothing is darkened

#### Scenario: Layer 1 is darkened and layer 2 is not, measured on the original
- **WHEN** layer 3 of cell (1, 1) is 15 and the tile 37 lies first in layer 1 and then in layer 2 of the cell
- **THEN** pixel (35, 40) shows (50, 12, 17), the texel B(163, 136) = (101, 26, 35) darkened, the first time and B(163, 136) the second time

#### Scenario: The edge is not darkened, measured on the original
- **WHEN** layer 1 of cell (1, 0) is 2844 with the flag 0x80, layer 0 of cell (1, 1) is 1545 and layer 3 of cell (1, 1) is 15
- **THEN** the pixels (35, 32) and (35, 39) show A2(35, 88) and A2(35, 95), and pixel (35, 40) shows (40, 11, 80)

#### Scenario: No shadow from walls, measured on the original
- **WHEN** layer 0 of the cells (0, 0) and (0, 1) is 4367 and layer 0 of cell (1, 1) is 2912, with no element of layer 3 set
- **THEN** the pixels (35, 40) and (52, 40) show A2(163, 72) and A2(148, 72)

### Requirement: Star tiles of RGSS3
In the rule set of RGSS3 the upper plane SHALL draw the tiles of layer 2 whose flags entry
has the bit 0x10, and the lower plane every other tile, among them the tiles of layers 0 and
1 that have the bit. No bit of a flags entry other than 0x10 and 0x80 SHALL change what is
drawn.

#### Scenario: A star tile in a lower layer, measured on the original
- **WHEN** the tile 300 with the flag 0x10 lies in layer 0 of cell (1, 1) of the small map, and again in layer 1
- **THEN** each time the lower plane alone shows C(131, 168) at pixel (35, 40), and the upper plane alone leaves the frame as the ground

#### Scenario: A star tile in layer 2, measured on the original
- **WHEN** the tile 300 with the flag 0x10 lies in layer 2 of cell (1, 1)
- **THEN** the upper plane alone shows C(131, 168) at pixel (35, 40), and the lower plane alone leaves the frame as the ground

#### Scenario: A star table on the upper plane, measured on the original
- **WHEN** layer 2 of cell (1, 0) is 2844 with the flag 0x90 and the corner of the op is (0, -3)
- **THEN** the upper plane alone shows A2(35, 71), A2(35, 88) at the pixels (35, 4), (35, 29), and the ground at pixel (35, 32); the lower plane alone leaves the frame as the ground

#### Scenario: The flag of number 0, measured on the original
- **WHEN** the small map is empty and element 0 of the flags table is 0x10
- **THEN** neither plane changes the frame

#### Scenario: Other bits, measured on the original
- **WHEN** the scenarios "A table by its flag" and "A star tile in layer 2" are composed with the flags 0x6f, 0x100 and 0x7f6f in place of 0x80 and 0x10
- **THEN** each frame equals the one composed with the flag 0

### Requirement: Undefined maps and numbers under the rule set of RGSS3
Where the original runtime of RGSS3 reads outside its arrays, a plane op of the rule set of
RGSS3 SHALL read nothing outside its tables: a number from 1024 to 1535 and a negative
number SHALL draw nothing, a layer the map does not have SHALL be empty, the shadow layer
among them, and a layer beyond the fourth SHALL NOT be read.

#### Scenario: Undefined numbers
- **WHEN** layer 0 of the cells (0, 0), (1, 0), (2, 0), (3, 0) and (0, 1) of the small map holds 1024, 1535, -256, -1 and 1300
- **THEN** the frame is the ground everywhere

#### Scenario: Fewer layers
- **WHEN** a map of 4 by 3 cells has three layers with 1545 in layer 0 of cell (1, 1), then one layer with the same, then two dimensions with the same
- **THEN** each time pixel (35, 40) shows A5(35, 40), nothing is darkened and no error is reported

#### Scenario: A fifth layer
- **WHEN** a map of 4 by 3 cells and five layers has 1545 in layer 0 of cell (1, 1), 37 in layer 4 of cell (1, 1) and 15 in layer 4 of cell (2, 1)
- **THEN** pixel (35, 40) shows A5(35, 40) and cell (2, 1) is the ground

### Requirement: Whole tilemaps of RGSS3 as measured
The CPU reference SHALL draw the topics `tables2`, `shadow3` and `edge1` and the first forty
frames of the topic `rand4` of the measurement of RGSS3, rebuilt by formula with generated
sheets, as the model of that measurement renders them. Their expected frames SHALL be
committed as digests that the model produced, not a compositor of this repository.

#### Scenario: Topics
- **WHEN** the 6 frames of `tables2`, the 22 of `shadow3`, the 48 of `edge1` and the first 40 of `rand4` are composed by the CPU reference
- **THEN** every frame has its committed digest

#### Scenario: Nothing of the original
- **WHEN** the files this change adds are listed
- **THEN** none is a picture, a DLL, a script of the publisher or map data of a game, and the file of digests holds only comments, topic names, frame numbers and digests

### Requirement: Pixels of a snapshot
The reference compositor SHALL offer the pixels of a snapshot of a packet that stands alone,
by a rule: by that of RGSS2 the composed frame, brightness applied, with alpha 255 in every
pixel; by that of RGSS3 the same, except that every byte of every pixel is 0 when the
brightness of the packet is 0. The colour channels SHALL be those of "Brightness of the
frame", and the alpha of the composed frame SHALL play no part.

#### Scenario: Darkened, measured on the originals
- **WHEN** a packet whose frame is the opaque colour (200, 100, 50) is snapped at brightness 100 by each rule
- **THEN** every pixel is (78, 39, 19, 255)

#### Scenario: Full brightness
- **WHEN** the same packet is snapped at brightness 255 by each rule
- **THEN** every pixel is (200, 100, 50, 255)

#### Scenario: Brightness 0 of RGSS3, measured on the original
- **WHEN** the same packet is snapped at brightness 0 by the rule of RGSS3, and at brightness 1
- **THEN** every pixel is (0, 0, 0, 0) in the first snapshot and (0, 0, 0, 255) in the second

#### Scenario: Brightness 0 of RGSS2
- **WHEN** the same packet is snapped at brightness 0 by the rule of RGSS2
- **THEN** every pixel is (0, 0, 0, 255), as `Graphics.snap_to_bitmap` of RGSS2 gave before this change

#### Scenario: Nothing drawn, measured on the original of RGSS3
- **WHEN** a packet without an op is snapped by the rule of RGSS3 at brightness 255, and at brightness 0
- **THEN** every pixel is (0, 0, 0, 255) in the first snapshot and (0, 0, 0, 0) in the second

#### Scenario: Translucent over nothing, measured on the original of RGSS3
- **WHEN** a packet that draws a quad of the texel (255, 255, 255, 128) at opacity 255 over nothing is snapped by the rule of RGSS3 at brightness 255
- **THEN** the pixels of the quad are (128, 128, 128, 255)

#### Scenario: Every level, measured on the original of RGSS3
- **WHEN** a packet whose frame holds every channel value from 0 to 255 is snapped by the rule of RGSS3 at each brightness from 1 to 254
- **THEN** every channel `v` is `(v * b) >> 8` and every alpha 255

#### Scenario: The scene as it is and the brightness of the call
- **WHEN** a scene shows a red sprite, a frame is extracted, the sprite's bitmap is made blue and the brightness set to 100 without another frame, and the scene's snapshot packet is snapped by the rule of RGSS3
- **THEN** the pixels of the sprite are (0, 0, 99, 255), and the next extracted frame is the one it would have been without the snapshot
