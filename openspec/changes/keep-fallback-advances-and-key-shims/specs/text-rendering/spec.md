# Spec Delta

## MODIFIED Requirements

### Requirement: Measurement
The width of a string SHALL be the sum of its characters' advance widths, each rounded to a
whole pixel at the font's whole pixels per em, without kerning or shaping, plus the amount by
which the first glyph reaches left of the pen. A character the selected font lacks and a
fallback font has SHALL be measured with the fallback font's advance at the same pixel size. A
line break, any other control character and a character no font has SHALL be measured as the
selected font's missing glyph and SHALL NOT start a new line. An empty string SHALL have width
0 and height 0.

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

#### Scenario: Fallback glyphs do not overlap
- **WHEN** two characters that only a fallback font has are drawn with a selected font whose missing-glyph advance is narrower than those glyphs
- **THEN** the second glyph starts where the first one's own advance ends, and the measured width is the sum of the two fallback advances
