# Spec Delta

## ADDED Requirements

### Requirement: Masked transition mix
The presenter SHALL be able to mix the held frame and the newest frame per pixel under a mask
bitmap, a progress from 0 to 1 and a vagueness from 1 to 256: with `m` the mask's red channel
at the pixel as a fraction, `v` the vagueness divided by 256 and `p` the progress, the weight
of the held frame SHALL be `(clamp(m, p, p + v) - p) / v`. The mask SHALL be stretched to the
frame. At progress 1 the output SHALL be the newest frame alone.

#### Scenario: Dark side first
- **WHEN** the mask is black on the left half and white on the right half, the vagueness is 1 and the progress is 0.5
- **THEN** the left half shows the newest frame and the right half the held frame

#### Scenario: Soft edge
- **WHEN** the mask is a horizontal gradient and the vagueness is 64
- **THEN** between the part showing the newest frame and the part showing the held frame there is a band in which the two are mixed

#### Scenario: End
- **WHEN** the progress is 1
- **THEN** the output equals the newest frame for any mask
