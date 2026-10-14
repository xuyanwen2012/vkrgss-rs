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
- **WHEN** `VKRGSS_RGSS3_FONTS` names a directory holding `VL-Gothic-Regular.ttf` and `VL-PGothic-Regular.ttf` and the committed table of `text_size` is evaluated with the rule of RGSS3
- **THEN** all 28392 widths and heights equal the measured values; without the variable the test says it is skipped

### Requirement: Metric rules
The engine SHALL measure and set text by one of two metric rules, set on the engine apart
from its mask rules: the plain rule, which is the default and gives every size, advance,
overhang and mask it gave before this requirement, and the rule of RGSS3. The same engine
with the same rules SHALL return identical results for identical requests.

#### Scenario: Default
- **WHEN** an engine whose metric rule was never set runs the existing tests of `vkrgss-text`
- **THEN** every expected value of those tests holds unedited

#### Scenario: Independent of the mask rules
- **WHEN** a string is measured and laid out with the metric rule of RGSS3 under each of the three mask rules
- **THEN** the three results are equal

#### Scenario: Switching back
- **WHEN** a string is laid out and rasterised with the plain metric rule, then with that of RGSS3, then with the plain rule again
- **THEN** the first and the third layout and mask are identical
