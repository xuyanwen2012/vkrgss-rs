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
The requested size SHALL be the height in pixels of the line box the text is set in, from its
ascent to its descent: the selected font's own line box, unless the line box of a replaced
family applies. The measured height of any string SHALL be that size. Bold and italic requests
SHALL select a bold, italic or bold-italic face of the family when one is available and
otherwise the regular face.

#### Scenario: Height equals size
- **WHEN** strings are measured at sizes 12, 22 and 52
- **THEN** their heights are 12, 22 and 52

#### Scenario: Bold face present
- **WHEN** a bold request is made for a family that has a bold face
- **THEN** the bold face is used and the string measures wider than in the regular face

#### Scenario: Bold face absent
- **WHEN** a bold request is made for a family with only a regular face
- **THEN** the regular face is used

### Requirement: Measurement
The width of a string SHALL be the sum of its characters' advance widths, each rounded to a
whole pixel, without kerning or shaping. An empty string SHALL have width 0. A line break or
other control character SHALL NOT start a new line.

#### Scenario: Additive widths
- **WHEN** the strings `A`, `B` and `AB` are measured
- **THEN** the width of `AB` is the sum of the other two

#### Scenario: Empty string
- **WHEN** an empty string is measured at size 22
- **THEN** the result is width 0 and height 22

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
