# Spec Delta

## ADDED Requirements

### Requirement: Blend arithmetic of the original on an opaque destination
For a quad drawn onto the frame, with `s` a source channel after tone and colour, `d` the
channel underneath and all values integers from 0 to 255, the compositor SHALL compute the
effective alpha as `a = (pixel_alpha * opacity + 127) / 255`, leave the pixel unchanged when
`a` is 0, and otherwise produce `d + (((s - d) * (a + 1)) >> 8)` in normal mode,
`min(255, d + ((s * (a + 1)) >> 8))` in additive mode and
`max(0, d - ((s * (a + 1)) >> 8))` in subtractive mode, with an arithmetic shift.

#### Scenario: Measured samples
- **WHEN** each committed sample measured on the original runtime for the three modes is evaluated
- **THEN** the result equals the measured value

#### Scenario: Opaque source
- **WHEN** pixel alpha and opacity are 255 in normal mode
- **THEN** the result equals the source channel

### Requirement: Colour and tone arithmetic of the original
The quad's colour SHALL change a channel `s` to `s + (((c - s) * (ca + 1)) >> 8)` for a colour
channel `c` with alpha `ca` above 0 and leave it unchanged for alpha 0, and the tone's red,
green and blue offsets SHALL change a channel to `s + t` clamped to 0 and 255, the tone before
the colour.

#### Scenario: Measured samples
- **WHEN** each committed colour, tone and combined sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value

### Requirement: Measured formulas are marked
The arithmetic functions that implement the two requirements above SHALL be listed with a
reverse-engineered provenance that names the measurement, and functions not yet measured SHALL
stay listed as placeholders.

#### Scenario: Provenance list
- **WHEN** the provenance of all pixel arithmetic functions is listed
- **THEN** the functions for blending onto an opaque destination, for the colour and for the tone offsets name the measurement, and at least the blend into a translucent destination is a placeholder
