# Spec Delta

## ADDED Requirements

### Requirement: Grey tone arithmetic of the original
The grey part of a tone SHALL be applied before its red, green and blue offsets: with
`L = (77 * r + 150 * g + 29 * b) >> 8`, each channel `c` SHALL become
`c + (((L - c) * (gray + 1)) >> 8)` for a grey value above 0 and stay unchanged for 0. CPU
reference and GPU SHALL agree bit for bit, and the function SHALL be listed with a
reverse-engineered provenance.

#### Scenario: Measured samples
- **WHEN** each committed tone sample measured on the original runtime is evaluated
- **THEN** the result equals the measured value
