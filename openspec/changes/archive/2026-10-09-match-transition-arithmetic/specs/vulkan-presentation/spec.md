# Spec Delta

## MODIFIED Requirements

### Requirement: Transition mix
The presenter SHALL be able to show a mix of a held frame and the newest frame with an integer
weight `an` from 0 to 255, computed per channel on the frames' 8-bit values before scaling as
`(old * (256 - an) + new * an) >> 8`, and SHALL show the newest frame unmixed when no
transition is active.

#### Scenario: End points
- **WHEN** the mix transition is inactive, and then active with weight 0
- **THEN** the output equals the held frame and then the newest frame

#### Scenario: Half way
- **WHEN** the held frame is black, the newest is white and the weight is one half
- **THEN** the output is mid-grey to within one step per channel

#### Scenario: Measured samples
- **WHEN** each committed cross-fade sample measured on the original runtime is evaluated with its weight
- **THEN** the output equals the measured value

### Requirement: Masked transition mix
The presenter SHALL be able to mix under a mask bitmap, a threshold `T` from 0 to 255 and a
vagueness: with `L` the mask's luminance at the pixel, the weight of the held frame SHALL be
`w = 0` for `L <= T` and `min(255, (L - T) * 255 / vague)` above it, or for a vagueness of 0
`255` for `L >= T` and 0 below, and the output SHALL be `new` for `w = 0` and
`new + (((old - new) * (w + 1)) >> 8)` otherwise. A mask smaller than the frame SHALL be tiled.

#### Scenario: Dark side first
- **WHEN** the mask is black on the left half and white on the right half, the vagueness is 1 and the progress is 0.5
- **THEN** the left half shows the newest frame and the right half the held frame

#### Scenario: Soft edge
- **WHEN** the mask is a horizontal gradient and the vagueness is 64
- **THEN** between the part showing the newest frame and the part showing the held frame there is a band in which the two are mixed

#### Scenario: End
- **WHEN** the progress is 1
- **THEN** the output equals the newest frame for any mask

#### Scenario: Measured samples
- **WHEN** each committed masked sample measured on the original runtime is evaluated with its threshold and vagueness
- **THEN** the output equals the measured value

#### Scenario: Tiled mask
- **WHEN** a 64 by 48 mask is used on a 640 by 480 frame
- **THEN** the pattern repeats every 64 pixels horizontally and every 48 vertically
