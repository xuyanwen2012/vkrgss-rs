## ADDED Requirements

### Requirement: Metric rules
The engine SHALL measure and set text by one of two metric rules, set on the engine apart
from its mask rules: the plain rule, which is the default and gives every size, advance,
overhang and mask it gave before this requirement, and the rule of RGSS3. The same engine
with the same rules SHALL return identical results for identical requests.

#### Scenario: Default
- **WHEN** an engine whose metric rule was never set runs the existing tests of `vkxp-text`
- **THEN** every expected value of those tests holds unedited

#### Scenario: Independent of the mask rules
- **WHEN** a string is measured and laid out with the metric rule of RGSS3 under each of the three mask rules
- **THEN** the three results are equal

#### Scenario: Switching back
- **WHEN** a string is laid out and rasterised with the plain metric rule, then with that of RGSS3, then with the plain rule again
- **THEN** the first and the third layout and mask are identical

### Requirement: Hinted outlines of RGSS3
With the metric rule of RGSS3 a glyph SHALL be measured and drawn from its outline hinted at
the whole pixels per em as FreeType hints it for an anti-aliased target without load flags:
by the automatic hinter for a TrueType font that has no `fpgm` table and a `prep` table of
at most 7 bytes, and by the font's own hinting engine otherwise.

#### Scenario: The engine of each test font
- **WHEN** the engine is chosen for Liberation Sans and for Noto Sans Symbols 2
- **THEN** it is the font's own hinting engine for the first and the automatic hinter for the second, which has neither table

#### Scenario: The size of the prep table
- **WHEN** the engine is chosen for copies of Liberation Sans without `fpgm` whose `prep` is 0, 7 and 8 bytes long
- **THEN** it is the automatic hinter for the first two and the font's own engine for the third

#### Scenario: Whole pixels
- **WHEN** every character of `Hello World` is laid out in both test fonts at the sizes 6 to 96 with the rule of RGSS3
- **THEN** every advance is a whole number of pixels equal to the hinted advance rounded up

### Requirement: Advances of RGSS3
With the metric rule of RGSS3 an advance SHALL be the hinted advance in whole pixels rounded
up. In a font whose OS/2 panose proportion is monospaced (9), with `a = xAvgCharWidth` and
`s = MulDiv(ppem, 65536, upm)`, a glyph whose advance equals `MulFix(2 * a, s)` SHALL be
`2 * MulFix(a, s)` wide. A synthesised bold SHALL add 1 to every advance.

#### Scenario: Full width in a fixed-pitch font, measured
- **WHEN** the rule is applied to a monospaced font of 1000 units per em and `xAvgCharWidth` 500 with a glyph whose hinted advance is 19 pixels at 19 pixels per em and 5 pixels at 5
- **THEN** the advances are 18 and 6, the widths of `あ` measured in VL Gothic at the sizes 24 and 6

#### Scenario: Not monospaced
- **WHEN** the same numbers are applied to a font whose panose proportion is not 9
- **THEN** the advances are 19 and 5

#### Scenario: Synthesised bold
- **WHEN** a string of five characters is laid out bold with the rule of RGSS3 in a family with only a regular face
- **THEN** each advance is the regular advance plus 1

### Requirement: Glyph box of RGSS3
With the metric rule of RGSS3 a glyph's box SHALL be the control box of its hinted outline
on whole pixels outwards, or for a synthesised bold that of the emboldened hinted outline as
it is; for a synthesised italic, the corners of that box slanted by `x + y / 4`. The left
overhang SHALL be minus the box's left edge rounded down, if positive; the right overhang
the box's right edge rounded up beyond the advance, if positive.

#### Scenario: Measured drawn widths
- **WHEN** `HHHHHHHHHH` at size 20 and `Hello World` in italic at size 33 are laid out in Liberation Sans with the rule of RGSS3
- **THEN** the width plus the last glyph's right overhang is 130 and 153, the drawn widths measured for "Arial", which was Liberation Sans

#### Scenario: The slanted box of an upright glyph
- **WHEN** the box from (0, -1) to (2, 5) pixels of a glyph that is not emboldened is slanted
- **THEN** its left edge is -0.25, and the left overhang is 1

#### Scenario: The slanted box of an emboldened glyph
- **WHEN** the box from (0.515625, -1) to (1.90625, 5.03125) pixels of an emboldened glyph is slanted
- **THEN** its left edge is 0.265625, and the left overhang is 0

### Requirement: Average width of RGSS3
With the metric rule of RGSS3 the average character width SHALL be
`MulFix(xAvgCharWidth, MulDiv(ppem, 65536, upm))`, GDI's `tmAveCharWidth` under Wine, at
least 1, plus 1 for a synthesised bold.

#### Scenario: Measured sizes of Liberation Sans
- **WHEN** the committed size samples of Liberation Sans are evaluated with the rule of RGSS3
- **THEN** pixels per em, ascent, descent and average width equal the measured values at every size

#### Scenario: VL Gothic, measured
- **WHEN** the rule is applied to 1000 units per em, `xAvgCharWidth` 500 and 19 pixels per em
- **THEN** the average width is 9, the value measured for VL Gothic at size 24, where the plain rule gives 10

### Requirement: Glyph images of RGSS3
With the metric rule of RGSS3 a glyph SHALL be rasterised from the hinted outline it is
measured by, emboldened, slanted and condensed as with the plain rule, and placed by the same
pen, so that its ink lies inside its box.

#### Scenario: Ink inside the box
- **WHEN** each character of `Hello World` is rasterised alone in both test fonts at the sizes 12, 24 and 48 in the four styles with the rule of RGSS3
- **THEN** no column of the mask left of the box's left edge or right of its right edge has ink

#### Scenario: Hinted, not plain
- **WHEN** `H` of a copy of Liberation Sans without `fpgm` and `prep` is rasterised at size 24 with each metric rule
- **THEN** the two masks differ

## MODIFIED Requirements

### Requirement: Text size measured on RGSS3
The measured size of a string with the metric rule of RGSS3 SHALL be the sum of its advances
by "Advances of RGSS3" plus the first glyph's left overhang by "Glyph box of RGSS3", and the
rounded ascent plus the rounded descent as "Size, bold and italic" gives them. With the
plain metric rule it SHALL stay as "Measurement" gives it.

#### Scenario: Measured on the original runtime of VX Ace
- **WHEN** `Hello`, `W` and `il` are measured at size 24 in the committed Liberation Sans with either metric rule
- **THEN** the sizes are 49 by 23, 20 by 23 and 10 by 23

#### Scenario: Synthesised bold
- **WHEN** a string of five characters is measured bold in a family with only a regular face
- **THEN** its width is the regular width plus 5 and its height is unchanged

#### Scenario: The fonts of the run-time package
- **WHEN** `VKXP_RGSS3_FONTS` names a directory holding `VL-Gothic-Regular.ttf` and `VL-PGothic-Regular.ttf` and the committed table of `text_size` is evaluated with the rule of RGSS3
- **THEN** all 28392 widths and heights equal the measured values; without the variable the test says it is skipped
