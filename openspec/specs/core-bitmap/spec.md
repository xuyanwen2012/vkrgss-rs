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
disposed bitmap as the source of a transfer SHALL NOT fail: every pixel of the destination
rectangle inside the destination SHALL be blended onto itself at the given opacity by the
arithmetic of the block transfer.

#### Scenario: Disposing twice
- **WHEN** a disposed bitmap is disposed again
- **THEN** nothing happens and no error is reported

#### Scenario: Drawing after disposal
- **WHEN** a pixel of a disposed bitmap is written
- **THEN** the operation fails with a disposed error

#### Scenario: Disposed source
- **WHEN** a disposed bitmap is used as the source of a block transfer at opacity 255 onto pixels of alpha 0, 128 and 255
- **THEN** no error is reported, the colours are unchanged and the alphas are 0, 191 and 255

#### Scenario: Samples of a disposed source
- **WHEN** each committed sample of a transfer from a disposed source measured on the originals is evaluated
- **THEN** every channel of every pixel equals the measured value

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
- **WHEN** the row of pixels `a b c d e f` is transferred within its bitmap from columns 0 to 3 to columns 1 to 4 at opacity 255
- **THEN** the row is `a a a a a f`, and a transfer from columns 1 to 4 to columns 0 to 3 of the same row gives `b c d e e f`

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
as 0. `Color` SHALL limit red, green and blue to -255 to 255 and alpha to 0 to 255 on
construction and assignment, and `Tone` SHALL keep its limits.

#### Scenario: Negative red
- **WHEN** `Color.new(-5, 0, 0)` is created and written with `set_pixel`
- **THEN** the colour's red is `-5.0` and the stored pixel's red is 251

#### Scenario: Upper bound
- **WHEN** `Color.new(300, -5, 0)` and `Tone.new(300, -300, 0)` are created
- **THEN** their first two components are 255 and -5, and 255 and -255

#### Scenario: Lower bound, measured on the originals
- **WHEN** `Color.new(-300, 0, 0, -5)` is created and its red is then set to -1000
- **THEN** its red is `-255.0` both times and its alpha `0.0`

### Requirement: Text squeezing
Text whose drawn width exceeds the rectangle SHALL be set again in a narrower form of the font
at the first of the steps 90, 80, 70 and 60 percent whose drawn width fits, each character's
advance being `ceil(natural * k / ave)` with `k = (size * step * ave) div (10 * th)` for the
step numbers 9 to 6, `ave` the font's rounded average character width and `th` the text
height.

#### Scenario: Measured levels
- **WHEN** each committed squeeze sample measured on the original runtime is evaluated
- **THEN** the drawn widths at the four steps equal the measured ones

### Requirement: Transfers within one bitmap work in place
A block transfer or stretched block transfer whose source and destination are the same bitmap
SHALL write the destination rows from top to bottom and the pixels of a row from left to
right, reading each source pixel as the bitmap is at that moment, pixels already written by
the same transfer included.

#### Scenario: Measured transfers
- **WHEN** each committed case measured on the original runtime (shifts in every direction, both opacities, stretched to the same size, to twice and to half the size, a mirrored transfer) is carried out
- **THEN** every pixel of the bitmap equals the measured one

### Requirement: Blur of RGSS2
The blur SHALL do nothing to a bitmap narrower or lower than 3 pixels, and otherwise SHALL
replace every channel of every pixel, alpha included, by the sum of that channel over the 3 by
3 pixels around it, read from the bitmap as it was and with the edge pixel repeated beyond the
edges, divided by 9 with truncation.

#### Scenario: Measured samples
- **WHEN** each committed blur sample measured on the original runtime of VX is evaluated
- **THEN** every channel of every pixel equals the measured value

#### Scenario: Transparent neighbours count with their colour
- **WHEN** a 3 by 3 bitmap holds (90, 0, 0, 0) everywhere except an opaque black centre and is blurred
- **THEN** the centre is (80, 0, 0, 28)

#### Scenario: Small bitmap
- **WHEN** a bitmap of 2 by 10 is blurred
- **THEN** it is unchanged

### Requirement: Radial blur of RGSS2
The radial blur SHALL follow rule 2 of this change's design: the angle clamped to 0 to 360 and
the division to 2 to 100, the samples rotated about (w >> 1, h >> 1) in double precision with
the constant 6.28318, mirrored at the edges, taken as nearest pixels of the bitmap as it was,
and each channel the truncated mean over the division. A bitmap narrower or lower than 3
pixels SHALL stay unchanged.

#### Scenario: Measured samples
- **WHEN** each committed radial blur sample measured on the original runtime of VX is evaluated
- **THEN** every channel of every pixel equals the measured value

#### Scenario: Limits
- **WHEN** a bitmap is blurred with `(1000, 1)` and a copy with `(360, 2)`
- **THEN** the two results are equal

### Requirement: Gradient of RGSS2
The gradient fill SHALL write to column i of the rectangle, or to row i when vertical, the
value `(c1 + (i * (c2 - c1)) / (n - 1)) & 255` for each channel, alpha included, with
truncating division, n being the width or the height, SHALL do nothing for n below 2, and
SHALL overwrite and clip as the fill does.

#### Scenario: Measured samples
- **WHEN** each committed gradient sample measured on the original runtime of VX is evaluated
- **THEN** every channel of every pixel equals the measured value

#### Scenario: End points
- **WHEN** a rectangle of width 4 is filled from (0, 0, 0, 255) to (255, 10, 0, 0)
- **THEN** the columns are (0, 0, 0, 255), (85, 3, 0, 170), (170, 6, 0, 85) and (255, 10, 0, 0)

#### Scenario: One column
- **WHEN** a rectangle of width 1 is filled horizontally
- **THEN** the bitmap is unchanged

### Requirement: Clearing a rectangle
Clearing a rectangle SHALL set every pixel of it that lies in the bitmap to (0, 0, 0, 0) and
SHALL do nothing for a width or height of 0 or less.

#### Scenario: Clipped
- **WHEN** the rectangle (-2, -2, 4, 4) of an opaque bitmap is cleared
- **THEN** the four pixels from (0, 0) to (1, 1) are transparent and all others unchanged

### Requirement: Text shadow of RGSS2
With the shadow on, text drawing SHALL first blend, one pixel to the right and one down, a
source of colour black and alpha `(cov * a + 127) / 255` for each pixel of the text cell, cov
being the coverage of the glyphs there and a the alpha of the font colour, by the arithmetic of
the block transfer, clipped by the bitmap only, and SHALL then draw the text as without a
shadow. A colour alpha of 0 SHALL draw nothing. The measured size of a text SHALL NOT count
the shadow.

#### Scenario: Layer from the coverage
- **WHEN** a text is drawn in white with the shadow on over a transparent bitmap, and its coverage is known from a draw without the shadow
- **THEN** every pixel equals the shadow layer of that coverage moved by (1, 1) with the text blended over it

#### Scenario: Outside the rectangle
- **WHEN** a text fills its rectangle to the right and bottom edge and the bitmap is larger
- **THEN** shadow pixels are drawn one pixel beyond the right and bottom edge of the rectangle

#### Scenario: Shadow off
- **WHEN** the same text is drawn with the shadow off
- **THEN** the result equals the draw of RGSS1

### Requirement: Text squeezing of RGSS2
In RGSS2 a text wider than its rectangle SHALL be narrowed by the loop `r = 1.0; while the
text is wider than the rectangle and r > 0.6: r -= 0.1` in double precision, which gives the
steps 90, 80, 70, 60 and 50 percent, and at the last step SHALL be drawn from the left edge
and cut off. RGSS1 SHALL keep the rule of "Text squeezing".

#### Scenario: Fifth step
- **WHEN** a text needs 55 percent of its natural width to fit
- **THEN** it is drawn at the 50 percent step in RGSS2 and at the last step of RGSS1 in RGSS1

#### Scenario: Steps, measured on the original
- **WHEN** rectangles of decreasing width are given for one text
- **THEN** the chosen step changes at the widths the loop gives, through all five steps

### Requirement: Opacity limit and mirrored clip of a transfer
A transfer with an opacity above 255 SHALL draw nothing. A transfer of equal sizes with a
negative source width or height SHALL be clipped before it is mirrored: per axis the low end
of the source span is cut at 0 and the destination span moved with it, then the destination is
cut at 0, then the source at its size, then the destination at its size, and what remains is
drawn mirrored within itself.

#### Scenario: Opacity above 255, measured on the originals
- **WHEN** a rectangle is transferred at opacity 256, 300 and 65536
- **THEN** the destination is unchanged each time

#### Scenario: Mirrored and cut, measured on the originals
- **WHEN** each committed sample of a mirrored transfer of equal sizes whose source rectangle leaves the source bitmap is evaluated, along each axis and along both
- **THEN** every pixel equals the measured value

### Requirement: Text rules by runtime
Drawing text SHALL take a value that names the runtime whose measured rules it follows:
RGSS1, RGSS2 or RGSS3. With RGSS1 and RGSS2 every draw SHALL give the pixels it gave before
this value existed, and the existing entry points that take no such value SHALL keep their
results. The requirements named "of RGSS3" SHALL apply with RGSS3 only.

#### Scenario: RGSS1 and RGSS2 are unchanged
- **WHEN** the text tests of the core, of the text engine and of the binding that existed before this change are run
- **THEN** all pass without a change to their expected values

#### Scenario: The same draw by three rules
- **WHEN** a text that fits its rectangle is drawn in a font without shadow and outline by the rules of RGSS1 and of RGSS2
- **THEN** the two bitmaps are equal

#### Scenario: RGSS3 touches more
- **WHEN** a text 9 wide and 24 high is drawn at (10, 10, 200, 60) of a 300 by 100 bitmap filled with (7, 8, 9, 0), in the probe's colours, by the rules of RGSS1 and of RGSS3, without outline
- **THEN** RGSS1 changes pixels of 9 columns and 24 rows only, and RGSS3 changes the columns 10 to 209 of the rows 28 to 87

### Requirement: Text placement of RGSS3
In RGSS3 a text SHALL be placed by `E` and `th`: its drawn width and its height, each 2
larger when the font has the outline. Its top SHALL be `y + max(0, (h - th) / 2)`. Its left
SHALL be `x` when left aligned, `x + max(0, (w - E + o) / 2)` when centred, with `o` 1 for a
font with the outline and 0 otherwise, and `max(x, x + w - E)` when right aligned. Divisions
truncate toward zero.

#### Scenario: Without outline, measured
- **WHEN** a text 9 wide and 24 high is drawn at (0, 0, 800, 600) of an 800 by 600 bitmap without outline, left, centred and right aligned
- **THEN** the top is 288 and the left is 0, 395 and 791

#### Scenario: Outline moves the text up, measured
- **WHEN** the same text is drawn at (10, 10, 200, 60) without and with the outline
- **THEN** the top is 28 without and 27 with it

#### Scenario: Outline when aligned
- **WHEN** a text 9 wide is drawn at x 0 in rectangles 100 and 101 wide, without and with the outline
- **THEN** right aligned the left is 91 and 92 without and 89 and 90 with it, and centred it is 45 and 46 without and 45 and 45 with it

#### Scenario: Rectangle lower than the text
- **WHEN** a text 24 high is drawn with the outline in a rectangle 25 high at y 10
- **THEN** the top is 10

#### Scenario: Text wider than the rectangle
- **WHEN** a text is still wider than its rectangle after the squeeze
- **THEN** its left is the rectangle's left at every alignment

### Requirement: Work rectangle of RGSS3
In RGSS3 the draw SHALL blend the work rectangle, not the text cell: every pixel (u, v) with
`0 <= u < min(w, 640)` and `0 <= v < min(h, 480)`, at (left + u, top + v), with the coverage
the text has there and 0 where it has none. It SHALL be cut by the bitmap only and SHALL NOT
depend on earlier draws. A width or height of 0 or less, or an empty string, SHALL draw
nothing.

#### Scenario: Measured boxes
- **WHEN** each committed line of the edge probes measured on the original runtime of VX Ace is drawn with a text 9 wide and 24 high on a bitmap filled with (7, 8, 9, 0), in the probe's colours
- **THEN** the box of the pixels that changed equals the measured one

#### Scenario: Below and right of the rectangle, measured
- **WHEN** such a text is drawn at (0, 0, 800, 600) of an 800 by 600 bitmap filled with (7, 8, 9, 0), in the probe's colours, centred, without outline
- **THEN** the changed pixels are the columns 395 to 799 of the rows 288 to 599

#### Scenario: Limits of 640 and 480, measured
- **WHEN** such a text is drawn left aligned without outline at (0, 0, 800, 100) of an 800 by 100 bitmap and at (5, -600, 40, 1200) of a 60 by 700 bitmap, both filled with (7, 8, 9, 0), in the probe's colours
- **THEN** the first changes the columns 0 to 639 and the second the rows 0 to 467

#### Scenario: Pixels without ink
- **WHEN** a work rectangle covers a pixel of alpha 0 and a pixel of alpha 255 where the text has no ink
- **THEN** the first takes the font colour with alpha 0 and the second is unchanged

#### Scenario: No state between draws
- **WHEN** a small draw is made before and after a draw with a rectangle of 800 by 600, on bitmaps of equal content
- **THEN** the two small draws change the same pixels to the same values

#### Scenario: Nothing to draw, measured
- **WHEN** a text is drawn at (10, 10, -5, 60) and at (10, 10, 100, 0), and an empty string in any rectangle
- **THEN** no pixel changes

### Requirement: Text passes of RGSS3
In RGSS3 the work rectangle SHALL be blended in passes by the arithmetic of the block
transfer, a pass at (dx, dy) in a colour at an alpha `a` giving the pixel at (u, v) the source
alpha `(cov * a + 127) / 255` and a pass of alpha 0 doing nothing. Without the outline: the
shadow pass (1, 1) in black if the font has the shadow, then the text pass (0, 0), both at the
alpha of the font colour. With it, "Outline of RGSS3" SHALL apply.

#### Scenario: Measured draws of the solid glyphs
- **WHEN** each committed draw of the glyphs "■" and "|" measured on the original runtime of VX Ace is drawn from the coverage of its draw without outline
- **THEN** every channel of every pixel of the box equals the measured value

#### Scenario: Shadow without outline
- **WHEN** a text is drawn by the rules of RGSS2 and of RGSS3 with the shadow, without outline, in a rectangle exactly as large as its cell on bitmaps of equal content
- **THEN** the two bitmaps are equal

#### Scenario: Shadow outside the work rectangle
- **WHEN** a text with ink in the last column and row of its work rectangle is drawn with the shadow and without outline
- **THEN** shadow pixels are drawn one pixel right of and below the work rectangle

#### Scenario: Colour alpha 0
- **WHEN** a text is drawn with a font colour of alpha 0, with outline and shadow on
- **THEN** no pixel changes

### Requirement: Outline of RGSS3
With the outline, RGSS3 SHALL blend only the pixels of the work rectangle with
`1 <= u < w - 1` and `1 <= v < h - 1`, in this order: four passes in the out colour at the
alpha `out alpha * colour alpha / 255`, at (-1, -1), (1, -1), (1, 1) and (-1, 1); the shadow
pass if the font has the shadow; the text pass. The glyphs SHALL keep their offset from
(left, top).

#### Scenario: One pass and two, measured
- **WHEN** a block of full coverage is drawn in white with the out colour (0, 0, 0, 128) on a transparent bitmap
- **THEN** a pixel that only one outline pass reaches has alpha 128, and a pixel beside a straight edge, which two reach, has alpha 191, both black

#### Scenario: No straight passes
- **WHEN** a glyph that is one vertical line one pixel wide is drawn with the outline
- **THEN** the pixel straight above its top end has no ink of the outline

#### Scenario: Alpha of the outline, measured
- **WHEN** a block of full coverage is drawn on a transparent bitmap with the out colour (255, 0, 0, 255) and a font colour of alpha 128
- **THEN** a pixel that only one outline pass reaches is (255, 0, 0, 128)

#### Scenario: Out colour of alpha 0
- **WHEN** a text is drawn with the outline and an out colour of alpha 0
- **THEN** the pixels with ink equal those of the text pass alone, placed as with the outline

#### Scenario: Border columns are dropped
- **WHEN** a glyph with ink in column 0 of its cell is drawn left aligned with the outline
- **THEN** that column of ink is not drawn, and no outline pixel lies left of the rectangle's left edge

#### Scenario: Narrow and low rectangles, measured
- **WHEN** a text 9 wide and 24 high is drawn with the outline at (10, 10, 2, 60), (10, 10, 3, 60), (10, 10, 100, 2) and (10, 10, 100, 3) of a 300 by 100 bitmap filled with (7, 8, 9, 0), in the probe's colours
- **THEN** the first and third change nothing, the second changes columns 10 to 12 of rows 27 to 86, and the fourth columns 10 to 109 of rows 10 to 12

#### Scenario: Outline at the limit of 640, measured
- **WHEN** such a text is drawn with the outline at (0, 0, 800, 600) of an 800 by 600 bitmap filled with (7, 8, 9, 0), in the probe's colours
- **THEN** the changed pixels are the columns 0 to 640 of the rows 287 to 599

#### Scenario: Shadow between outline and text
- **WHEN** a block of full coverage is drawn on a transparent bitmap in opaque white with the out colour (255, 0, 0, 255), with and without the shadow
- **THEN** the pixel diagonally below and right of the block's lower right corner is opaque black with the shadow and opaque red without it, and the block is white in both

### Requirement: Text squeezing of RGSS3
In RGSS3 a text whose `E` exceeds the width `w` of the rectangle SHALL be set again once, in
a font of the average character width `trunc(size * r * 7 / 16)` with
`r = max(0.6, w / E - 0.15)`, computed in double precision in this order, and SHALL be drawn
with the width that gives, fitting or not. `E` SHALL include the outline's 2 in the test and
in `r`. A squeezed `E` of 0 or less SHALL draw nothing. RGSS1 and RGSS2 SHALL keep their rules.

#### Scenario: Measured widths asked for
- **WHEN** each committed pair of a rectangle width and the width asked for, from the 24 sweeps measured on the original runtime of VX Ace, is evaluated for its size and `E`
- **THEN** the width asked for equals the measured one, for all 2330 pairs

#### Scenario: Size 20, measured
- **WHEN** a text with `E` 130 at size 20 is drawn in rectangles 130, 129, 124, 123, 109, 108 and 12 wide
- **THEN** it is not squeezed at 130, and the widths asked for are 7, 7, 6, 6, 5 and 5

#### Scenario: Outline squeezes earlier, measured
- **WHEN** a text with a drawn width of 108 at size 24 is drawn in rectangles 109, 101, 100 and 89 wide, without and with the outline
- **THEN** without it the text is not squeezed at 109 and the widths asked for are 8, 8 and 7; with it they are 8, 8, 7 and 6

#### Scenario: One step only
- **WHEN** a text needs half its natural width to fit
- **THEN** it is set once at the width asked for with `r` 0.6, starts at the rectangle's left edge and ends at the work rectangle

#### Scenario: Squeezed text is placed by its own width
- **WHEN** a squeezed text that fits is drawn right aligned with the outline
- **THEN** its left is `x + w - E` with `E` the squeezed drawn width plus 2

### Requirement: Text measurement of RGSS3
In RGSS3 the measured size of a text SHALL be the one "Text measurement" gives, the same with
the outline or the shadow on or off: neither the outline's 2 nor the last glyph's right
overhang SHALL be counted.

#### Scenario: Outline and shadow
- **WHEN** a text is measured with a font of RGSS3 with outline and shadow off, and again with both on
- **THEN** the two sizes are equal

#### Scenario: Empty string
- **WHEN** an empty string is measured with a font that has the outline
- **THEN** the width is 0 and the height is 0

### Requirement: Largest bitmap of RGSS3
The bitmap arena SHALL apply to a game of RGSS3 the limit it has for every bitmap, 268435456
pixels, and no limit of 4194304 pixels.

#### Scenario: Above the limit of RGSS2
- **WHEN** a bitmap of 2049 by 2048 is created in the arena
- **THEN** it is created

#### Scenario: The limit
- **WHEN** the arena's limit is read, and a bitmap of 32768 by 32768 is requested
- **THEN** the limit is 268435456 and the request fails with a creation error
