# Spec Delta

## MODIFIED Requirements

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
