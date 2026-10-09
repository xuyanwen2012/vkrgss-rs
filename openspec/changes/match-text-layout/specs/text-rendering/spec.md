# Spec Delta

## MODIFIED Requirements

### Requirement: Size, bold and italic
The requested size SHALL be the height in pixels of the cell the text is set in. The font SHALL
be set at a whole number of pixels per em: `round(upm * size / box)`, lowered by one when
`round(box * ppem / upm)` exceeds the size, where `box` is the selected font's own ascent plus
descent, or the line box of a replaced family when that applies. The measured height of any
non-empty string SHALL be the rounded ascent plus the rounded descent at that size. Bold and
italic requests SHALL select a matching face when one is available and otherwise synthesise
the style from the regular face.

#### Scenario: Height equals size
- **WHEN** strings are measured at sizes 12, 22 and 52 in a font whose line box equals its em
- **THEN** their heights are 12, 22 and 52

#### Scenario: Height from rounded metrics, measured
- **WHEN** each committed size sample measured on the original runtime for the two test fonts is evaluated
- **THEN** pixels per em and text height equal the measured values, for example 15 and 21 at size 22 in the first font

#### Scenario: Bold face present
- **WHEN** a bold request is made for a family that has a bold face
- **THEN** the bold face is used and the string measures wider than in the regular face

#### Scenario: Bold face absent
- **WHEN** a bold request is made for a family with only a regular face
- **THEN** the regular face is used with every character one pixel wider, and an italic request keeps the regular advances

### Requirement: Measurement
The width of a string SHALL be the sum of its characters' advance widths, each rounded to a
whole pixel at the font's whole pixels per em, without kerning or shaping, plus the amount by
which the first glyph reaches left of the pen. A character the font lacks, a line break and
any other control character SHALL be measured as the font's missing glyph and SHALL NOT start
a new line. An empty string SHALL have width 0 and height 0.

#### Scenario: Additive widths
- **WHEN** the strings `A`, `B` and `AB` are measured
- **THEN** the width of `AB` is the sum of the other two

#### Scenario: Empty string
- **WHEN** an empty string is measured at size 22
- **THEN** the result is width 0 and height 0

#### Scenario: Measured widths
- **WHEN** each committed width sample measured on the original runtime is evaluated
- **THEN** the width equals the measured value

#### Scenario: Left overhang
- **WHEN** a string that starts with a glyph reaching left of the pen is measured
- **THEN** its width is the sum of the advances plus that overhang
