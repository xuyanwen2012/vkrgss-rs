# Spec Delta

## MODIFIED Requirements

### Requirement: Hue change
A hue change SHALL convert every pixel to the original runtime's integer hue, saturation and
lightness form, add the angle, truncated to an integer and raised by 360 once when negative,
to the hue, reduce the sum by 360 once when it reaches 360, and convert back, with the
measured integer arithmetic, leaving alpha unchanged. An angle of 0 SHALL change nothing.

#### Scenario: Full turn
- **WHEN** a bitmap's hue is changed by 0 degrees
- **THEN** no pixel changes; and a change by 360 degrees gives the values measured on the original runtime, which differ from the input for some colours

#### Scenario: Primary colours
- **WHEN** a pure red pixel is rotated by 120 degrees and another by 240 degrees
- **THEN** the results equal the values the measured arithmetic gives, which are green and blue up to its rounding

#### Scenario: Grey is unaffected
- **WHEN** a pixel with equal red, green and blue is rotated by any angle
- **THEN** it equals the value the measured arithmetic gives for a pixel of saturation 0

#### Scenario: Alpha is kept
- **WHEN** a half-transparent pixel is rotated
- **THEN** its alpha is unchanged

#### Scenario: Measured samples
- **WHEN** each committed hue sample measured on the original runtime is evaluated
- **THEN** red, green and blue equal the measured values
