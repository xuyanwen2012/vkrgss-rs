# text-rendering Specification

## Purpose
Defines how vkxp finds a font for the family names a game asks for and turns a string into
glyph coverage at a pixel size, including what happens when the requested font or a glyph is
missing.

## Requirements

### Requirement: Font sources
The text engine SHALL take its fonts from the fonts installed on the system and from
directories given by the caller. When the same family exists in both, the caller's directory
SHALL win. It SHALL also be constructible from caller directories alone, without system fonts.

#### Scenario: Caller directory only
- **WHEN** the engine is built from a directory holding one font and without system fonts
- **THEN** exactly that font's family is available

#### Scenario: Caller directory wins
- **WHEN** a family is installed on the system and a different file of the same family name is in a caller directory
- **THEN** text in that family is drawn with the caller's file

### Requirement: Family lookup
A request names families in order of preference. Each name SHALL be matched without regard to
letter case, first against the available families and then through a substitution table that
maps a name to an ordered list of replacements. The first name that yields an available family
SHALL be used; when none does, the engine's fallback fonts SHALL be used.

#### Scenario: First available name
- **WHEN** the request is `["NoSuchFont", "Liberation Sans"]` and only the second is available
- **THEN** Liberation Sans is used

#### Scenario: Substitution
- **WHEN** the request is `["Arial"]`, Arial is not available and the table maps Arial to Liberation Sans
- **THEN** Liberation Sans is used

#### Scenario: Case
- **WHEN** the request is `["liberation sans"]`
- **THEN** Liberation Sans is used

#### Scenario: Nothing matches
- **WHEN** no requested name is available or substitutable
- **THEN** text is still drawn, with a fallback font

### Requirement: Default substitutions
The built-in substitution table SHALL cover at least the Windows families `Arial`, `Tahoma`,
`Verdana`, `Times New Roman`, `Courier New`, `MS Gothic`, `MS PGothic`, `MS Mincho`, `SimSun`,
`宋体`, `SimHei`, `黑体`, `KaiTi`, `楷体`, `楷体_GB2312`, `Microsoft YaHei` and `微软雅黑`, each
mapped to openly licensed families commonly installed on Linux, and the caller SHALL be able to
add entries that take precedence over the built-in ones.

#### Scenario: Chinese family name
- **WHEN** `黑体` is looked up in the built-in table
- **THEN** the replacements include a Chinese sans-serif family such as Noto Sans CJK SC

#### Scenario: Caller entry takes precedence
- **WHEN** the caller maps `Arial` to another family that is available
- **THEN** a request for `Arial` uses the caller's family

### Requirement: Availability query
The engine SHALL report whether a family name is available, directly or through the
substitution table.

#### Scenario: Substituted name counts as available
- **WHEN** `Arial` is queried, is not installed and is substituted by an installed family
- **THEN** the answer is yes

#### Scenario: Unknown name
- **WHEN** a name that is neither installed nor substitutable is queried
- **THEN** the answer is no

### Requirement: Per-character fallback
When the selected font has no glyph for a character, that character SHALL be drawn from the
first fallback font that has one, at the same pixel size, and the rest of the string SHALL stay
in the selected font. A character that no font has SHALL take the width of the selected font's
missing-glyph box and SHALL NOT cause an error.

#### Scenario: Mixed string
- **WHEN** a string mixes characters the selected font has with one only a fallback font has
- **THEN** every character produces coverage, and the measured width is the sum of each character's advance in the font it came from

#### Scenario: No font has the character
- **WHEN** a string contains a character that no available font has
- **THEN** measuring and rasterising succeed

### Requirement: Size, bold and italic
The requested size SHALL be the height in pixels of the cell the text is set in. The font SHALL
be set at a whole number of pixels per em: `round(upm * size / box)`, lowered by one when
`round(box * ppem / upm)` exceeds the size, where `box` is the selected font's own ascent plus
descent, or the line box of a replaced family when that applies. The measured height of any
non-empty string SHALL be the rounded ascent plus the rounded descent at that size.

#### Scenario: Height equals size
- **WHEN** strings are measured at sizes 12, 22 and 52 in a font whose line box equals its em
- **THEN** their heights are 12, 22 and 52

#### Scenario: Height from rounded metrics, measured
- **WHEN** each committed size sample measured on the original runtime for the two test fonts is evaluated
- **THEN** pixels per em and text height equal the measured values, for example 15 and 21 at size 22 in the first font

### Requirement: Bold and italic faces
Bold and italic requests SHALL select a matching face when one is available and otherwise
synthesise the style from the regular face.

#### Scenario: Bold face present
- **WHEN** a bold request is made for a family that has a bold face
- **THEN** the bold face is used and the string measures wider than in the regular face

#### Scenario: Bold face absent
- **WHEN** a bold request is made for a family with only a regular face
- **THEN** the regular face is used with every character one pixel wider, and an italic request keeps the regular advances

### Requirement: Measurement
The width of a string SHALL be the sum of its characters' advance widths, each rounded to a
whole pixel at the font's whole pixels per em, without kerning or shaping, plus the amount by
which the first glyph reaches left of the pen. A
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

### Requirement: Advance of fallback glyphs
A character that the selected font lacks and a fallback font has SHALL be measured, and set
when drawn, with the fallback font's advance at the same pixel size.

#### Scenario: Fallback glyphs do not overlap
- **WHEN** two characters that only a fallback font has are drawn with a selected font whose missing-glyph advance is narrower than those glyphs
- **THEN** the second glyph starts where the first one's own advance ends, and the measured width is the sum of the two fallback advances

### Requirement: Rasterisation
Rasterising a string SHALL return a coverage mask, one byte per pixel from 0 (none) to 255
(full), of exactly the measured width and height, with each glyph placed at the sum of the
advances before it and on the font's baseline. Coverage outside the mask SHALL be clipped, not
an error. The same engine SHALL return identical masks for identical requests.

#### Scenario: Mask size
- **WHEN** a string is measured and rasterised with the same request
- **THEN** the mask has the measured width and height

#### Scenario: Ink where expected
- **WHEN** the letter `I` is rasterised at size 40
- **THEN** the mask has full coverage somewhere in its middle columns and none in its corner pixels

#### Scenario: Space
- **WHEN** a single space is rasterised
- **THEN** the mask has a positive width and no coverage

#### Scenario: Repeatable
- **WHEN** the same request is rasterised twice, before and after other requests
- **THEN** the two masks are byte-identical

### Requirement: Line box of a replaced family
When the selected font stands in for a requested family through the substitution table and the
line box of that family is known, the font SHALL be scaled and placed by the replaced family's
line box instead of its own: one em is the size divided by the replaced family's ascent plus
descent in ems, and the baseline lies the replaced family's share of ascent below the top.

#### Scenario: Replacement takes the size of the family it replaces
- **WHEN** `SimHei` is not available, a caller entry replaces it by Liberation Sans, its line box is known to be one em high, and an em dash is measured at size 22
- **THEN** the em dash is 22 pixels wide, wider than when Liberation Sans is requested by name

#### Scenario: Height stays the size
- **WHEN** the same request measures any string at size 22
- **THEN** the height is 22

### Requirement: Known line boxes
The engine SHALL have a built-in list of line boxes for Windows families of its substitution
table, at least `Arial`, `SimSun`, `宋体`, `SimHei`, `黑体`, `MS Gothic` and `MS Mincho`, looked up
without regard to letter case, and the caller SHALL be able to give line boxes that take
precedence over the built-in ones.

#### Scenario: Built-in Chinese family
- **WHEN** `黑体` is not available and is replaced by an available family, and a string is measured at size 22
- **THEN** one em of the replacement is 22 pixels

#### Scenario: Arial agrees with its metric-compatible replacement
- **WHEN** the built-in line box of `Arial` is compared with the own line box of Liberation Sans
- **THEN** they are equal

#### Scenario: Caller's line box takes precedence
- **WHEN** the caller gives `Arial` a line box of one em and Arial is replaced by Liberation Sans
- **THEN** an em dash measured at size 22 is 22 pixels wide

### Requirement: Own line box otherwise
A font found under the requested name, a replacement for a family whose line box is not known,
and a fallback font SHALL keep their own line box.

#### Scenario: Unknown family
- **WHEN** a family without a known line box is replaced by Liberation Sans
- **THEN** a string measures exactly as when Liberation Sans is requested by name

#### Scenario: Installed family
- **WHEN** a family with a known line box is itself available
- **THEN** its own font is used with its own line box

### Requirement: Own line box on request
When the engine is configured to use fonts' own metrics, a font found for a requested name,
whether by that name or as a replacement, SHALL be sized so that its own line box (ascent plus
descent) equals the requested size, and no scaling towards the line box of a replaced font
SHALL be applied. Without that configuration the behaviour SHALL be unchanged.

#### Scenario: Replacement font at its own size
- **WHEN** a replaced Windows font name resolves to a test font and text of size 22 is measured with own metrics on
- **THEN** the measured height is 22 and the width equals the width measured when the test font is requested by its own name

#### Scenario: Default unchanged
- **WHEN** the same text is measured without the configuration
- **THEN** the result equals what it was before this change

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
