# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
