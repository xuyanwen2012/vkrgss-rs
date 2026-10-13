# core-values Specification

## Purpose
Defines the value types that scripts manipulate (colour, tone, rectangle, table and font
attributes): their ranges, equality and the byte layout of their serialized forms, which must
match the data files of the original runtime.

## Requirements

### Requirement: Colour components
A colour SHALL hold red, green, blue and alpha as floating-point numbers; whenever a component
is set, red, green and blue SHALL be clamped to the range -255 to 255 and alpha to 0 to 255.
A colour created without an alpha SHALL have alpha 255. Two colours SHALL be equal exactly
when all four components are equal.

#### Scenario: Components are clamped
- **WHEN** a colour is given red 300, green -300 and alpha -7
- **THEN** it reports red 255, green -255 and alpha 0

#### Scenario: Negative components are kept
- **WHEN** a colour is given green -5
- **THEN** it reports green -5

#### Scenario: Fractions are kept
- **WHEN** a colour is given blue 12.5
- **THEN** it reports blue 12.5

#### Scenario: Default alpha
- **WHEN** a colour is created from red, green and blue only
- **THEN** its alpha is 255

#### Scenario: Equality compares every component
- **WHEN** two colours differ only in alpha
- **THEN** they are not equal

### Requirement: Tone components
A tone SHALL hold red, green and blue as floating-point numbers clamped to -255 to 255 and a
grey amount clamped to 0 to 255, clamping whenever a component is set. A tone created without a
grey amount SHALL have grey 0. Two tones SHALL be equal exactly when all four components are
equal.

#### Scenario: Components are clamped
- **WHEN** a tone is given red -300, blue 400 and grey -1
- **THEN** it reports red -255, blue 255 and grey 0

#### Scenario: Default grey
- **WHEN** a tone is created from red, green and blue only
- **THEN** its grey amount is 0

### Requirement: Rectangle
A rectangle SHALL hold x, y, width and height as 32-bit signed integers and store them as given,
including negative widths and heights. Emptying a rectangle SHALL set all four to 0. Two
rectangles SHALL be equal exactly when all four values are equal.

#### Scenario: Values are stored as given
- **WHEN** a rectangle is set to x -10, y 20, width -30, height 0
- **THEN** it reports exactly those values

#### Scenario: Emptying
- **WHEN** a rectangle with non-zero values is emptied
- **THEN** x, y, width and height are all 0

### Requirement: Conversion of components to integers
When a colour or tone is used for drawing or sent to the host, each component SHALL be converted
to an integer by discarding its fractional part. The red, green and blue of a colour SHALL then
be taken modulo 256, and an alpha below 0 SHALL be 0; a tone's components do not wrap.

#### Scenario: Fraction is discarded
- **WHEN** a colour with red 127.9 is converted for drawing
- **THEN** red is 127

#### Scenario: Negative tone component
- **WHEN** a tone with red -0.9 is converted for drawing
- **THEN** red is 0

#### Scenario: Negative colour component wraps
- **WHEN** colours with red -1, -5, -100.7 and -0.5 are converted for drawing
- **THEN** red is 255, 251, 156 and 0

### Requirement: Table
A table SHALL be a one-, two- or three-dimensional array of signed 16-bit integers, all zero when
created. Reading outside its bounds SHALL yield no value and writing outside its bounds SHALL
change nothing. A written value SHALL keep only its low 16 bits, read back as a signed number.

#### Scenario: New table is zero
- **WHEN** a table of 4 by 3 by 2 is created
- **THEN** every one of its 24 elements reads 0

#### Scenario: Reading outside the bounds
- **WHEN** an element is read at an index equal to the size of its dimension, or at a negative index
- **THEN** no value is returned

#### Scenario: Writing outside the bounds
- **WHEN** an element is written at an index outside the table
- **THEN** no element changes

#### Scenario: Values wrap to 16 bits
- **WHEN** 40000 is written to an element
- **THEN** the element reads -25536

#### Scenario: Unused dimensions have size one
- **WHEN** a one-dimensional table of 17 elements is created
- **THEN** its x size is 17 and its y and z sizes are 1

### Requirement: Table resize
Resizing a table SHALL keep the value of every element whose coordinates exist both before and
after, and SHALL set every new element to 0.

#### Scenario: Growing keeps the contents
- **WHEN** a 2 by 2 table holding four different values is resized to 3 by 3
- **THEN** the four original elements keep their values at their coordinates and the five new ones are 0

#### Scenario: Shrinking drops the rest
- **WHEN** a 3 by 3 table is resized to 2 by 1
- **THEN** the two remaining elements keep their values

### Requirement: Serialized form of colour, tone and rectangle
A colour and a tone SHALL serialize to 32 bytes: their four components as little-endian 64-bit
floating-point numbers in the order red, green, blue, then alpha or grey. A rectangle SHALL
serialize to 16 bytes: x, y, width and height as little-endian 32-bit signed integers.
Deserializing SHALL restore an equal value and SHALL fail on input of any other length.

#### Scenario: Known colour bytes
- **WHEN** the colour with all four components 255 is serialized
- **THEN** the result is the eight bytes `00 00 00 00 00 E0 6F 40` repeated four times

#### Scenario: Round trip
- **WHEN** any colour, tone or rectangle is serialized and deserialized
- **THEN** the result equals the original

#### Scenario: Wrong length
- **WHEN** 31 bytes are deserialized as a colour
- **THEN** deserialization fails

### Requirement: Serialized form of table
A table SHALL serialize to five little-endian 32-bit signed integers (the number of dimensions,
the x, y and z sizes, and the element count) followed by its elements as little-endian 16-bit
signed integers with x varying fastest, then y, then z. Deserializing SHALL restore an equal
table and SHALL fail when the element count does not equal the product of the sizes or the
input length does not match.

#### Scenario: Map-sized table
- **WHEN** a 20 by 15 by 3 table is serialized
- **THEN** the result is 1820 bytes long and starts with the integers 3, 20, 15, 3, 900

#### Scenario: Element order
- **WHEN** a 2 by 2 table with distinct values is serialized
- **THEN** the elements appear in the order (0,0), (1,0), (0,1), (1,1)

#### Scenario: Inconsistent header
- **WHEN** input whose element count field differs from the product of its sizes is deserialized
- **THEN** deserialization fails

### Requirement: Font attributes
A font value SHALL hold an ordered list of family names, a size, a bold flag, an italic flag and
a colour. A new font SHALL take every attribute that is not given explicitly from the defaults
in force when it is created, and changing the defaults afterwards SHALL NOT change existing
fonts. Until changed, the defaults SHALL be size 22, not bold, not italic and opaque white.

#### Scenario: New font copies the defaults
- **WHEN** the default size is changed to 30 and a font is then created without a size
- **THEN** the font's size is 30

#### Scenario: Existing fonts keep their values
- **WHEN** a font is created and the default colour is changed afterwards
- **THEN** the font's colour is unchanged

#### Scenario: Initial defaults
- **WHEN** a font is created before any default is changed
- **THEN** its size is 22, it is neither bold nor italic and its colour is opaque white

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
