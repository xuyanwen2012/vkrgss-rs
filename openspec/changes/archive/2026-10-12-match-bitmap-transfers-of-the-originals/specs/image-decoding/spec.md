## MODIFIED Requirements

### Requirement: BMP
BMP decoding SHALL support uncompressed images of 1, 4, 8, 24 and 32 bits per pixel, stored
bottom-up or top-down. Images of 32 bits SHALL use their fourth byte as alpha, also when it is
0 in every pixel; all other depths SHALL be opaque. Compressed BMP data SHALL fail with an
unsupported-format error.

#### Scenario: Bottom-up rows
- **WHEN** a bottom-up 24-bit BMP whose first stored row is red and second is blue is decoded
- **THEN** the top row of the result is blue and the bottom row is red

#### Scenario: Palette
- **WHEN** an 8-bit BMP with a two-colour palette is decoded
- **THEN** each pixel has the colour of its palette entry and alpha 255

#### Scenario: Thirty-two bits without alpha
- **WHEN** a 32-bit BMP whose fourth bytes are all 0 is decoded
- **THEN** every pixel has alpha 0
