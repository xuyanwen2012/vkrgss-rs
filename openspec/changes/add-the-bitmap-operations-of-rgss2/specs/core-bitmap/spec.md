## ADDED Requirements

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
