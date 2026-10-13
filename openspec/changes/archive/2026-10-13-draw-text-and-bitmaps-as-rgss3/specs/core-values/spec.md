## ADDED Requirements

### Requirement: Outline attributes of a font
A font value SHALL also hold an outline flag and an out colour, and the font defaults SHALL
hold both; a new font SHALL copy them from the defaults in force as it copies the other
attributes. The defaults of RGSS1 and of RGSS2 SHALL have the outline off.

#### Scenario: Copied from the defaults
- **WHEN** the default outline is turned off and the default out colour set to (1, 2, 3, 4), and a font is then created
- **THEN** the font has no outline and the out colour (1, 2, 3, 4)

#### Scenario: Existing fonts keep their values
- **WHEN** a font is created and the default out colour is changed afterwards
- **THEN** the font's out colour is unchanged

#### Scenario: RGSS1 and RGSS2
- **WHEN** a font is created from the initial defaults of RGSS1 and one from those of RGSS2
- **THEN** neither has the outline, and their other attributes are what they were before this requirement

### Requirement: Font defaults of RGSS3
The initial font defaults of RGSS3 SHALL be the one family name `VL Gothic`, size 24, not
bold, not italic, no shadow, the outline on, the colour (255, 255, 255, 255) and the out
colour (0, 0, 0, 128).

#### Scenario: Measured on the original
- **WHEN** a font is created from the initial defaults of RGSS3
- **THEN** its names are `["VL Gothic"]`, its size is 24, bold, italic and shadow are off, the outline is on, its colour is (255, 255, 255, 255) and its out colour is (0, 0, 0, 128)

### Requirement: Empty colour and tone, and copies between them
A colour and a tone created without components SHALL have all four components 0. A colour
SHALL be convertible to a tone and a tone to a colour by copying the four numbers in order,
the alpha becoming the gray and the gray the alpha.

#### Scenario: Without components, measured on the original
- **WHEN** a colour, a tone and a rectangle are created without components
- **THEN** each has four zeros

#### Scenario: Copy between the classes, measured on the original
- **WHEN** the tone (1, 2, 3, 4) is converted to a colour and the colour (1, 2, 3, 4) to a tone
- **THEN** the results are the colour (1, 2, 3, 4) and the tone (1, 2, 3, 4)

#### Scenario: Fractions and negative components
- **WHEN** the colour (-9, 255, 2.5, 77) is converted to a tone and back
- **THEN** the result equals the colour
