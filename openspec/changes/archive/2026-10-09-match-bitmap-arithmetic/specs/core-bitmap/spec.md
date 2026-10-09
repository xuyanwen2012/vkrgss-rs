# Spec Delta

## ADDED Requirements

### Requirement: Blend arithmetic of blt as in the original
`blt` and `stretch_blt` SHALL combine a source pixel (channel `s`, alpha `sa`) with a
destination pixel (`d`, `da`) in integer arithmetic: `a = (sa * opacity + 127) / 255`;
`den = 255 * da + 255 * a - da * a`; `w = a * 65025 / den`, lowered by one for the five
measured pairs of `(a, da)`; the colour is `d` when `w` is 0 and
`d + (((s - d) * (w + 1)) >> 8)` otherwise; the alpha is `den / 255`. Opacity 0 SHALL leave
the destination unchanged.

#### Scenario: Measured samples
- **WHEN** each committed `blt` sample measured on the original runtime is evaluated
- **THEN** the resulting red, green, blue and alpha equal the measured values

#### Scenario: Nothing underneath and nothing drawn
- **WHEN** `den` is 0, that is `a` and `da` are both 0
- **THEN** the result is the source colour with alpha 0

#### Scenario: Opaque destination
- **WHEN** the destination alpha is 255
- **THEN** the colour equals the screen blend formula for alpha `a` and the alpha stays 255

### Requirement: Stretch sampling as in the original
`stretch_blt` SHALL not interpolate. For a destination rectangle of width `dw` and a source
rectangle of width `sw`, destination column `i`, counted from the unclipped left edge, SHALL
take source column `sx + ((i * ((sw << 20) / dw)) >> 20)`, and rows likewise. A negative
source width SHALL mirror, in `blt` as well; a negative destination size SHALL draw nothing;
and source rectangles that leave the source bitmap SHALL follow the measured clipping rules.

#### Scenario: Measured samples
- **WHEN** each committed stretch sample measured on the original runtime is evaluated
- **THEN** every destination pixel equals the measured value

#### Scenario: Sixteen to twenty-four
- **WHEN** a 16-pixel row is stretched to 24 pixels
- **THEN** the first six destination pixels take source columns 0, 0, 1, 1, 2, 3

### Requirement: Colour components stored as in the original
A colour component written by `fill_rect`, `set_pixel` or text drawing SHALL be truncated
toward zero and taken modulo 256 for red, green and blue, and an alpha below 0 SHALL be stored
as 0. `Color` and `Tone` SHALL clamp only the upper bound of their components on construction
and assignment, keeping negative colour components.

#### Scenario: Negative red
- **WHEN** `Color.new(-5, 0, 0)` is created and written with `set_pixel`
- **THEN** the colour's red is `-5.0` and the stored pixel's red is 251

#### Scenario: Upper bound
- **WHEN** `Color.new(300, -5, 0)` and `Tone.new(300, -300, 0)` are created
- **THEN** their first two components are 255 and -5, and 255 and -255
