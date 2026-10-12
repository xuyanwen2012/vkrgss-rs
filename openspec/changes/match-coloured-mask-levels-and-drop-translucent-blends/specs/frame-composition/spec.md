## MODIFIED Requirements

### Requirement: Measured formulas are marked
The arithmetic functions that implement the two requirements above SHALL be listed with a
reverse-engineered provenance that names the measurement, and functions not yet measured SHALL
stay listed as placeholders. The additive and the subtractive blend SHALL each have one
formula, the measured one, for every destination, and SHALL keep the destination's alpha.

#### Scenario: Provenance list
- **WHEN** the provenance of all pixel arithmetic functions is listed
- **THEN** the functions for blending, for the colour and for the tone offsets name the measurement, and no blend function is a placeholder

#### Scenario: Destination with alpha
- **WHEN** the colour (200, 100, 255) with alpha 128 is added with opacity 255 to the destination (10, 20, 30) with alpha 100
- **THEN** the result is (110, 70, 158) with alpha 100
