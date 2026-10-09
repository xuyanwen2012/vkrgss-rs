# Spec Delta

## Purpose
Defines how the contents of PNG, JPEG and BMP files become bitmap pixels in the layout the
protocol uses: red, green, blue and straight alpha, one byte each, in row order from the top.

## ADDED Requirements

### Requirement: Format is recognised by content
The decoder SHALL recognise PNG, JPEG and BMP input by its leading bytes, whatever the file was
named, and SHALL fail with an unsupported-format error for anything else.

#### Scenario: Wrong extension
- **WHEN** PNG data taken from a file named with a `.jpg` extension is decoded
- **THEN** it decodes as PNG

#### Scenario: Unknown format
- **WHEN** the input starts with bytes of none of the three formats
- **THEN** decoding fails with an unsupported-format error

### Requirement: Decoded image
A successful decode SHALL return the width, the height and exactly `width * height * 4` bytes:
red, green, blue and straight alpha per pixel, rows from top to bottom. Sources without
transparency SHALL decode with alpha 255.

#### Scenario: Opaque source
- **WHEN** an image without an alpha channel is decoded
- **THEN** every pixel has alpha 255

#### Scenario: Size of the result
- **WHEN** an image of 3 by 2 pixels is decoded
- **THEN** 24 bytes are returned

### Requirement: PNG
PNG decoding SHALL support greyscale, greyscale with alpha, palette, truecolour and truecolour
with alpha, at every bit depth the format allows, interlaced or not. Transparency given by a
`tRNS` chunk SHALL become alpha, 16-bit samples SHALL be reduced to their high byte, and no gamma
or colour-profile correction SHALL be applied.

#### Scenario: Lossless round trip
- **WHEN** 8-bit truecolour-with-alpha pixels are encoded as PNG and decoded
- **THEN** the decoded pixels equal the original ones byte for byte

#### Scenario: Palette with transparency
- **WHEN** a palette image whose `tRNS` chunk makes one entry fully transparent is decoded
- **THEN** pixels using that entry have alpha 0 and the others alpha 255

#### Scenario: Sixteen-bit samples
- **WHEN** a 16-bit greyscale sample of value 0xABCD is decoded
- **THEN** red, green and blue are 0xAB

### Requirement: JPEG
JPEG decoding SHALL support baseline and progressive images in greyscale and in three-component
colour, with chroma at full resolution or subsampled by two horizontally, vertically or both,
and SHALL return them opaque. A JPEG outside what the decoder supports, such as one with another
number of components, SHALL fail with an unsupported-format error.

#### Scenario: Greyscale
- **WHEN** a greyscale JPEG is decoded
- **THEN** red, green and blue are equal in every pixel and alpha is 255

#### Scenario: Colour
- **WHEN** a JPEG made from a flat mid-grey image with subsampled chroma is decoded
- **THEN** every channel of every pixel is within 2 of the original value

#### Scenario: Unsupported layout
- **WHEN** a JPEG with four components is decoded
- **THEN** decoding fails with an unsupported-format error

### Requirement: BMP
BMP decoding SHALL support uncompressed images of 1, 4, 8, 24 and 32 bits per pixel, stored
bottom-up or top-down. Images of 32 bits SHALL use their fourth byte as alpha when at least one
pixel has it non-zero and SHALL be opaque otherwise; all other depths SHALL be opaque.
Compressed BMP data SHALL fail with an unsupported-format error.

#### Scenario: Bottom-up rows
- **WHEN** a bottom-up 24-bit BMP whose first stored row is red and second is blue is decoded
- **THEN** the top row of the result is blue and the bottom row is red

#### Scenario: Palette
- **WHEN** an 8-bit BMP with a two-colour palette is decoded
- **THEN** each pixel has the colour of its palette entry and alpha 255

#### Scenario: Thirty-two bits without alpha
- **WHEN** a 32-bit BMP whose fourth bytes are all 0 is decoded
- **THEN** every pixel has alpha 255

### Requirement: Malformed input
Decoding SHALL fail with a typed error and SHALL NOT panic when the input is truncated or
cannot be decoded, and a decode that succeeds SHALL return an image of the full declared size.
A damaged file that still decodes MAY be accepted, with what it lacks filled in, as common
decoders do; detecting every deviation from a format's standard is not required. An image
whose pixel count exceeds the largest bitmap the compositors accept SHALL be refused before
its pixels are allocated.

#### Scenario: Truncated files
- **WHEN** valid PNG, JPEG and BMP inputs are cut off at every length shorter than the original
- **THEN** each decode either fails with an error or returns a complete image of the declared size

#### Scenario: Random bytes
- **WHEN** arbitrary byte strings that start with a valid signature are decoded
- **THEN** no decode panics

#### Scenario: Oversized image
- **WHEN** a header declares 65535 by 65535 pixels
- **THEN** decoding fails with a size error without allocating the pixels
