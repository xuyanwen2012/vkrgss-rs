## ADDED Requirements

### Requirement: Level of a mask pixel
The level `L` of a mask pixel with the colour (r, g, b) SHALL be
`(5014911 * r + 9841699 * g + 1920606 * b) >> 24`, computed without overflow, whatever the
pixel's alpha.

#### Scenario: Grey
- **WHEN** the level of the pixel (v, v, v) is computed for every v from 0 to 255
- **THEN** it is v

#### Scenario: Measured colours
- **WHEN** each committed `lum` sample measured on the original runtime is presented as a mask pixel
- **THEN** the mask value equals the measured level, also for the colours (5, 231, 0) with 137 and (0, 3, 203) with 24
