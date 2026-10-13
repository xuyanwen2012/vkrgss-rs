## ADDED Requirements

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
