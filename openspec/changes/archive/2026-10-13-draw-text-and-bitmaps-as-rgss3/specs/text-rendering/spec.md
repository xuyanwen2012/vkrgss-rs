## ADDED Requirements

### Requirement: Mask rules
The engine SHALL make its masks by one of three rules, set on the engine: the plain rule,
which is the default and gives every mask it gave before this requirement, the rule of RGSS3
and the early rule of RGSS3. Measured sizes and advances SHALL NOT depend on the rule. The
same engine with the same rule SHALL return identical masks for identical requests.

#### Scenario: Default
- **WHEN** an engine whose rule was never set rasterises the strings of the existing raster tests
- **THEN** the masks are those of before, and overlapping glyphs keep the larger coverage

#### Scenario: Sizes do not depend on the rule
- **WHEN** a string is measured and laid out with each of the three rules
- **THEN** the three results are equal

#### Scenario: Switching back
- **WHEN** a string is rasterised with the plain rule, then with the rule of RGSS3, then with the plain rule again
- **THEN** the first and the third mask are byte-identical

### Requirement: Coverage levels of RGSS3
With either rule of RGSS3 the coverage of a glyph pixel SHALL first become a level `v` from
0 to 64 and the mask SHALL hold `trunc(v * 255 / 64)`. A coverage of 0 SHALL give level 0,
full coverage level 64, and a larger coverage SHALL never give a smaller level.

#### Scenario: Scaled levels, measured on the original
- **WHEN** the levels 0, 1, 32, 49, 56, 63 and 64 are scaled
- **THEN** the mask values are 0, 3, 127, 195, 223, 251 and 255

#### Scenario: End points
- **WHEN** a glyph with pixels of coverage 0 and 255 is composed into a mask with the rule of RGSS3
- **THEN** the mask holds 0 and 255 there

#### Scenario: Only scaled values
- **WHEN** a string of the test font is rasterised at size 24 with the rule of RGSS3
- **THEN** every byte of the mask is `trunc(v * 255 / 64)` for some `v` from 0 to 64, and the mask has ink

### Requirement: Overlapping glyphs of RGSS3
With the rule of RGSS3 a glyph composed into the mask SHALL replace what earlier glyphs wrote
wherever its own level is above 0 and SHALL leave the mask as it is elsewhere. With the early
rule it SHALL replace every pixel of its own box, also where its level is 0.

#### Scenario: Later ink replaces, version 3.0.1.1
- **WHEN** a glyph of full coverage is followed by a glyph whose box lies over it and holds a level between 0 and 64 in one pixel and 0 in another
- **THEN** the first pixel holds the later glyph's value, which is smaller, and the second stays 255

#### Scenario: The box erases, version 3.0.0.1
- **WHEN** the same two glyphs are composed with the early rule
- **THEN** the first pixel holds the later glyph's value and the second is 0

#### Scenario: Outside the box
- **WHEN** either rule composes the second glyph
- **THEN** the pixels of the first glyph outside the second one's box are unchanged

### Requirement: Exact family names
The engine SHALL answer whether a name exists exactly: whether it equals, character for
character and with regard to letter case, a family name that an available font declares, or
a name of the substitution table as the table spells it, one of whose replacements is
available. This question SHALL NOT change "Availability query" or "Family lookup".

#### Scenario: Letter case
- **WHEN** `Liberation Sans`, `liberation sans` and `LIBERATION SANS` are asked with the test font available
- **THEN** the answers are yes, no and no

#### Scenario: Not a family name
- **WHEN** `Liberation Sans Regular`, `LiberationSans-Regular`, the empty string and a single space are asked
- **THEN** every answer is no

#### Scenario: Substituted name
- **WHEN** `Arial` and `arial` are asked, Arial is not installed and the table replaces it by the test font
- **THEN** the answers are yes and no

#### Scenario: The other question is unchanged
- **WHEN** `liberation sans` is asked of the availability query
- **THEN** the answer is yes, as before

### Requirement: Name of a name list in RGSS3
The engine SHALL choose from an ordered list of names the first that exists exactly, the
first of the list when none does, and none for an empty list.

#### Scenario: First existing
- **WHEN** the list is `NoSuchFont`, `Liberation Sans`, `Noto Sans Symbols 2` with both test fonts available
- **THEN** the second name is chosen

#### Scenario: None exists
- **WHEN** the list is `NoSuchFont`, `NoSuchFont2`
- **THEN** the first name is chosen

#### Scenario: Empty list
- **WHEN** the list is empty
- **THEN** no name is chosen

### Requirement: Text size measured on RGSS3
The measured size of a string SHALL be the same for a font of RGSS3 as "Measurement" and
"Size, bold and italic" give it: the sum of the advances plus the first glyph's left
overhang, and the rounded ascent plus the rounded descent.

#### Scenario: Measured on the original runtime of VX Ace
- **WHEN** `Hello`, `W` and `il` are measured at size 24 in the committed Liberation Sans
- **THEN** the sizes are 49 by 23, 20 by 23 and 10 by 23

#### Scenario: Synthesised bold
- **WHEN** a string of five characters is measured bold in a family with only a regular face
- **THEN** its width is the regular width plus 5 and its height is unchanged
