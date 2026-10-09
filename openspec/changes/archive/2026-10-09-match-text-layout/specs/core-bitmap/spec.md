# Spec Delta

## MODIFIED Requirements

### Requirement: Text drawing
Drawing text SHALL place a string inside a rectangle using the bitmap's font and a text
rasteriser supplied by the embedder: its top at `y + max(0, (h - th) div 2)` for a text height
`th`, aligned left, centre or right by the drawn width, which is the measured width plus the
last glyph's right overhang, and cut to the rectangle and the bitmap. The whole text cell
SHALL be blended with the `blt` arithmetic at a source alpha of
`(coverage * colour_alpha + 127) / 255`. Text wider than the rectangle SHALL be squeezed as
the requirement "Text squeezing" says.

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

#### Scenario: Coverage and colour alpha
- **WHEN** the font colour has alpha 0
- **THEN** nothing is drawn; and with a colour alpha above 0, a pixel of the text cell with coverage 0 is unchanged where the destination's alpha is above 0 and takes the font colour with alpha 0 where it is 0

#### Scenario: Rectangle lower than the text
- **WHEN** text of height 21 is drawn in a rectangle of height 10
- **THEN** the top 10 rows of the text cell appear, not its middle

#### Scenario: Measured placement
- **WHEN** each committed vertical and alignment sample measured on the original runtime is drawn
- **THEN** the rows and columns of the text cell equal the measured ones

### Requirement: Text measurement
Measuring text SHALL return the width and height the rasteriser reports for the string in the
bitmap's font, without drawing. An empty string SHALL have width 0 and height 0.

#### Scenario: Measurement matches drawing
- **WHEN** a string without a right overhang is measured and then drawn left aligned in a rectangle of exactly that size
- **THEN** no squeezing occurs and no glyph pixel falls outside the rectangle

#### Scenario: Empty string
- **WHEN** an empty string is measured
- **THEN** the width is 0 and the height is 0

## ADDED Requirements

### Requirement: Text squeezing
Text whose drawn width exceeds the rectangle SHALL be set again in a narrower form of the font
at the first of the steps 90, 80, 70 and 60 percent whose drawn width fits, each character's
advance being `ceil(natural * k / ave)` with `k = (size * step * ave) div (10 * th)` for the
step numbers 9 to 6, `ave` the font's rounded average character width and `th` the text
height.

#### Scenario: Measured levels
- **WHEN** each committed squeeze sample measured on the original runtime is evaluated
- **THEN** the drawn widths at the four steps equal the measured ones
