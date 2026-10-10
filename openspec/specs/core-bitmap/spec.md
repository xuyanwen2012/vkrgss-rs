# core-bitmap Specification

## Purpose
Defines the CPU bitmap that scripts draw into: its pixel format, drawing operations, disposal,
text drawing through a rasteriser interface, and how its changes reach the host as bitmap
deltas.

## Requirements

### Requirement: Bitmap creation
A bitmap SHALL be created with a width and a height of at least 1 pixel and SHALL start with
every pixel transparent black. It SHALL also be creatable from existing pixels given as red,
green, blue and straight alpha bytes in row order. Creation SHALL fail with a creation error
when a side is 0, when the pixel count exceeds the largest bitmap the compositors accept, or
when given pixels do not match the size.

#### Scenario: New bitmap is transparent
- **WHEN** a bitmap of 4 by 3 is created
- **THEN** all twelve pixels have red, green, blue and alpha 0

#### Scenario: Zero side
- **WHEN** a bitmap of width 0 is requested
- **THEN** creation fails with a creation error

#### Scenario: Too large
- **WHEN** a bitmap with more pixels than the compositors accept is requested
- **THEN** creation fails with a creation error and no memory for it stays allocated

#### Scenario: From pixels
- **WHEN** a bitmap is created from 2 by 2 pixels with four distinct values
- **THEN** each pixel reads back the value given for its position

### Requirement: Bitmap font
Every bitmap SHALL have a font value of its own, created from the font defaults in force when
the bitmap is created, and replaceable as a whole. Text drawn into the bitmap SHALL use that
font.

#### Scenario: Font comes from the defaults
- **WHEN** the default font size is 30 and a bitmap is created
- **THEN** the bitmap's font has size 30

#### Scenario: Fonts are not shared
- **WHEN** the font of one bitmap is changed
- **THEN** the font of another bitmap created from the same defaults is unchanged

### Requirement: Disposal
After a bitmap is disposed, every operation on it other than disposing it again and asking
whether it is disposed SHALL fail with a disposed error and SHALL change nothing. Using a
disposed bitmap as the source of a transfer SHALL fail in the same way.

#### Scenario: Disposing twice
- **WHEN** a disposed bitmap is disposed again
- **THEN** nothing happens and no error is reported

#### Scenario: Drawing after disposal
- **WHEN** a pixel of a disposed bitmap is written
- **THEN** the operation fails with a disposed error

#### Scenario: Disposed source
- **WHEN** a disposed bitmap is used as the source of a block transfer
- **THEN** the operation fails with a disposed error and the destination is unchanged

### Requirement: Pixel access
Reading a pixel SHALL return its colour, and reading outside the bitmap SHALL return the colour
with all four components 0. Writing a pixel SHALL replace all four of its channels with the
converted colour, and writing outside the bitmap SHALL change nothing.

#### Scenario: Write then read
- **WHEN** a pixel is written with a colour and read back
- **THEN** the colour read equals the colour written with its fractions discarded

#### Scenario: Reading outside
- **WHEN** a pixel at a negative coordinate or beyond the last column is read
- **THEN** the result is the colour (0, 0, 0, 0)

#### Scenario: Writing outside
- **WHEN** a pixel outside the bitmap is written
- **THEN** no pixel changes

### Requirement: Clear and fill
Clearing a bitmap SHALL set every pixel to transparent black. Filling a rectangle SHALL set
every pixel inside both the rectangle and the bitmap to the given colour, alpha included,
without blending. A rectangle with a width or height of 0 or less SHALL fill nothing.

#### Scenario: Fill replaces alpha
- **WHEN** an opaque bitmap is filled with a colour of alpha 0
- **THEN** the filled pixels have alpha 0 and the colour's red, green and blue

#### Scenario: Fill is clipped to the bitmap
- **WHEN** a rectangle that extends past the right and bottom edges is filled
- **THEN** only the pixels inside the bitmap change

#### Scenario: Clear
- **WHEN** a bitmap with arbitrary contents is cleared
- **THEN** every pixel is transparent black

### Requirement: Block transfer
A block transfer SHALL draw a rectangle of a source bitmap at a position in the destination with
an opacity from 0 to 255, combining each source pixel with the destination pixel by straight-alpha
source-over. Only the part of the source rectangle that lies inside the source bitmap SHALL be
drawn, at the offset it has within the rectangle, and only where it lands inside the destination.

#### Scenario: Opaque copy
- **WHEN** a fully opaque source rectangle is transferred at opacity 255
- **THEN** the destination pixels it covers equal the source pixels

#### Scenario: Zero opacity
- **WHEN** a rectangle is transferred at opacity 0
- **THEN** the destination is unchanged

#### Scenario: Onto a transparent destination
- **WHEN** a source pixel with alpha 128 is transferred at opacity 255 onto a transparent pixel
- **THEN** the destination pixel has the source's red, green and blue and alpha 128

#### Scenario: Source rectangle partly outside the source
- **WHEN** the source rectangle starts 2 pixels left of the source bitmap
- **THEN** its first two columns draw nothing and the third column lands 2 pixels right of the given position

#### Scenario: Source and destination are the same bitmap
- **WHEN** a rectangle is transferred within one bitmap to an overlapping position
- **THEN** the result equals transferring from an unchanged copy of the bitmap

### Requirement: Stretched block transfer
A stretched transfer SHALL draw a source rectangle scaled into a destination rectangle with
nearest sampling: destination column `i` of a destination width `w` takes source column
`floor(i * sw / w)` of a source width `sw`, and rows likewise. Blending, opacity and clipping
SHALL be those of the block transfer. An empty source or destination rectangle SHALL draw nothing.

#### Scenario: Same size equals a block transfer
- **WHEN** the destination rectangle has the size of the source rectangle
- **THEN** the result equals a block transfer to the destination rectangle's position

#### Scenario: Doubling
- **WHEN** a 2 by 1 source is stretched to 4 by 2
- **THEN** each source pixel appears as a 2 by 2 block

#### Scenario: Halving
- **WHEN** a 4 by 1 source is stretched to 2 by 1
- **THEN** the destination takes source columns 0 and 2

### Requirement: Hue change
A hue change SHALL convert every pixel to the original runtime's integer hue, saturation and
lightness form, add the angle, truncated to an integer and raised by 360 once when negative,
to the hue, reduce the sum by 360 once when it reaches 360, and convert back, with the
measured integer arithmetic, leaving alpha unchanged. An angle of 0 SHALL change nothing.

#### Scenario: Full turn
- **WHEN** a bitmap's hue is changed by 0 degrees
- **THEN** no pixel changes; and a change by 360 degrees gives the values measured on the original runtime, which differ from the input for some colours

#### Scenario: Primary colours
- **WHEN** a pure red pixel is rotated by 120 degrees and another by 240 degrees
- **THEN** the results equal the values the measured arithmetic gives, which are green and blue up to its rounding

#### Scenario: Grey is unaffected
- **WHEN** a pixel with equal red, green and blue is rotated by any angle
- **THEN** it equals the value the measured arithmetic gives for a pixel of saturation 0

#### Scenario: Alpha is kept
- **WHEN** a half-transparent pixel is rotated
- **THEN** its alpha is unchanged

#### Scenario: Measured samples
- **WHEN** each committed hue sample measured on the original runtime is evaluated
- **THEN** red, green and blue equal the measured values

### Requirement: Text drawing
Drawing text SHALL place a string inside a rectangle using the bitmap's font and a text
rasteriser supplied by the embedder: its top at `y + max(0, (h - th) div 2)` for a text height
`th`, aligned left, centre or right by the drawn width, which is the measured width plus the
last glyph's right overhang, and cut to the rectangle and the bitmap. The text cell SHALL be blended as "Text
blending" says, and text wider than the rectangle squeezed as "Text squeezing" says.

#### Scenario: Alignment
- **WHEN** text narrower than its rectangle is drawn left, centre and right aligned
- **THEN** its left edge is at the rectangle's left edge, half the free width in, and the full free width in

#### Scenario: Vertical centring
- **WHEN** text of height 10 is drawn in a rectangle of height 30
- **THEN** it occupies rows 10 to 19 of the rectangle

#### Scenario: Squeezing
- **WHEN** ten characters of natural advance 11 are drawn in a rectangle of width 100
- **THEN** they are set at the 90 percent step with whole-pixel advances and aligned by their own width, not scaled to 100 pixels

#### Scenario: Squeeze limit
- **WHEN** text is wider than the rectangle even at the 60 percent step
- **THEN** it is drawn at that step from the rectangle's left edge for every alignment, and clipped

#### Scenario: Rectangle lower than the text
- **WHEN** text of height 21 is drawn in a rectangle of height 10
- **THEN** the top 10 rows of the text cell appear, not its middle

#### Scenario: Measured placement
- **WHEN** each committed vertical and alignment sample measured on the original runtime is drawn
- **THEN** the rows and columns of the text cell equal the measured ones

### Requirement: Text blending
The whole text cell SHALL be blended into the bitmap with the `blt` arithmetic at a source
alpha of `(coverage * colour_alpha + 127) / 255`.

#### Scenario: Coverage and colour alpha
- **WHEN** the font colour has alpha 0
- **THEN** nothing is drawn; and with a colour alpha above 0, a pixel of the text cell with coverage 0 is unchanged where the destination's alpha is above 0 and takes the font colour with alpha 0 where it is 0

### Requirement: Text measurement
Measuring text SHALL return the width and height the rasteriser reports for the string in the
bitmap's font, without drawing. An empty string SHALL have width 0 and height 0.

#### Scenario: Measurement matches drawing
- **WHEN** a string without a right overhang is measured and then drawn left aligned in a rectangle of exactly that size
- **THEN** no squeezing occurs and no glyph pixel falls outside the rectangle

#### Scenario: Empty string
- **WHEN** an empty string is measured
- **THEN** the width is 0 and the height is 0

### Requirement: Bitmap identity
Every bitmap SHALL have a protocol bitmap id that no other bitmap, live or disposed, has had
during the life of the arena that created it.

#### Scenario: Slot reuse
- **WHEN** a bitmap is disposed and a new bitmap takes over its slot
- **THEN** the new bitmap's id differs from the disposed one's

### Requirement: Bitmap deltas
When asked for the deltas of a set of referenced bitmaps, the arena SHALL emit, in an order a
compositor accepts: a destroy for every disposed bitmap the host knows; a create and a full
update for each referenced bitmap the host does not know; an update of the rectangle changed
since its last update for each referenced bitmap the host knows; and nothing for an unchanged
one. A changed bitmap that is not referenced SHALL keep its pending change until it is.

#### Scenario: First reference
- **WHEN** a new bitmap is referenced for the first time
- **THEN** a create and an update covering the whole bitmap are emitted

#### Scenario: Unchanged bitmap
- **WHEN** a bitmap the host knows is referenced again without having changed
- **THEN** nothing is emitted for it

#### Scenario: Small change
- **WHEN** one pixel of a known 100 by 100 bitmap is written and the bitmap is referenced
- **THEN** one update is emitted whose rectangle contains that pixel and is smaller than the bitmap

#### Scenario: Change while unreferenced
- **WHEN** a known bitmap is changed, left out of one request and referenced in the next
- **THEN** the first request emits nothing for it and the second emits an update containing the change

#### Scenario: Disposed bitmaps
- **WHEN** a bitmap the host knows and a bitmap it never saw are both disposed
- **THEN** exactly one destroy is emitted, for the known one

### Requirement: Deltas reproduce the bitmap
Applying the emitted deltas, in order, to a compositor's bitmap store SHALL leave that store
holding, for every referenced bitmap, exactly the pixels the bitmap has.

#### Scenario: Random operation sequences
- **WHEN** random sequences of bitmap creation, drawing, disposal and delta requests are run and
  their deltas are applied to the reference compositor's bitmap store
- **THEN** after every request each referenced bitmap in the store equals the bitmap byte for byte

### Requirement: Bitmap arithmetic provenance
Each arithmetic function added for bitmap operations SHALL declare whether its formula is a
textbook placeholder or derived from a named reverse-engineering source, and those declarations
SHALL be queryable together, separately from the compositing functions that GPU backends mirror.

#### Scenario: Listing bitmap arithmetic
- **WHEN** the provenance of the bitmap arithmetic is listed
- **THEN** the hue rotation and the stretch index mapping appear, each with a placeholder marker
  or a named source, and no compositing function appears

### Requirement: Blend arithmetic of blt as in the original
`blt` and `stretch_blt` SHALL combine a source pixel (channel `s`, alpha `sa`) with a
destination pixel (`d`, `da`) in integer arithmetic: `a = (sa * opacity + 127) / 255`;
`den = 255 * da + 255 * a - da * a`; `w = a * 65025 / den`, lowered by one for the five
measured pairs of `(a, da)`; the colour is `d` when `w` is 0 and
`d + (((s - d) * (w + 1)) >> 8)` otherwise; the alpha is `den / 255`. Opacity 0 SHALL leave
the destination unchanged.

#### Scenario: Measured samples
- **WHEN** each committed `blt` sample measured on the original runtime is evaluated
- **THEN** the resulting red, green, blue and alpha equal the measured values

#### Scenario: Nothing underneath and nothing drawn
- **WHEN** `den` is 0, that is `a` and `da` are both 0
- **THEN** the result is the source colour with alpha 0

#### Scenario: Opaque destination
- **WHEN** the destination alpha is 255
- **THEN** the colour equals the screen blend formula for alpha `a` and the alpha stays 255

### Requirement: Stretch sampling as in the original
`stretch_blt` SHALL not interpolate. For a destination rectangle of width `dw` and a source
rectangle of width `sw`, destination column `i`, counted from the unclipped left edge, SHALL
take source column `sx + ((i * ((sw << 20) / dw)) >> 20)`, and rows likewise. A negative
source width SHALL mirror, in `blt` as well; a negative destination size SHALL draw nothing;
and source rectangles that leave the source bitmap SHALL follow the measured clipping rules.

#### Scenario: Measured samples
- **WHEN** each committed stretch sample measured on the original runtime is evaluated
- **THEN** every destination pixel equals the measured value

#### Scenario: Sixteen to twenty-four
- **WHEN** a 16-pixel row is stretched to 24 pixels
- **THEN** the first six destination pixels take source columns 0, 0, 1, 1, 2, 3

### Requirement: Colour components stored as in the original
A colour component written by `fill_rect`, `set_pixel` or text drawing SHALL be truncated
toward zero and taken modulo 256 for red, green and blue, and an alpha below 0 SHALL be stored
as 0. `Color` and `Tone` SHALL clamp only the upper bound of their components on construction
and assignment, keeping negative colour components.

#### Scenario: Negative red
- **WHEN** `Color.new(-5, 0, 0)` is created and written with `set_pixel`
- **THEN** the colour's red is `-5.0` and the stored pixel's red is 251

#### Scenario: Upper bound
- **WHEN** `Color.new(300, -5, 0)` and `Tone.new(300, -300, 0)` are created
- **THEN** their first two components are 255 and -5, and 255 and -255

### Requirement: Text squeezing
Text whose drawn width exceeds the rectangle SHALL be set again in a narrower form of the font
at the first of the steps 90, 80, 70 and 60 percent whose drawn width fits, each character's
advance being `ceil(natural * k / ave)` with `k = (size * step * ave) div (10 * th)` for the
step numbers 9 to 6, `ave` the font's rounded average character width and `th` the text
height.

#### Scenario: Measured levels
- **WHEN** each committed squeeze sample measured on the original runtime is evaluated
- **THEN** the drawn widths at the four steps equal the measured ones
