# Spec Delta

## ADDED Requirements

### Requirement: Scaling filters
The presenter SHALL scale the frame into its letterbox rectangle with one of three filters:
nearest, which shows for each output pixel the frame pixel under its centre; linear, which
interpolates between the four nearest frame pixels; and sharp bilinear, which equals nearest
except in a band one output pixel wide along each frame pixel boundary, where it interpolates.
The filter `auto` SHALL be nearest when the scale factor is a whole number and sharp bilinear
otherwise.

#### Scenario: Integer scale
- **WHEN** a 4 by 4 frame of distinct colours is presented at scale 3 with `auto`
- **THEN** every output pixel equals one frame pixel exactly, in 3 by 3 blocks

#### Scenario: Sharp bilinear at a fractional scale
- **WHEN** a frame of black and white vertical stripes one pixel wide is presented at scale 2.5 with sharp bilinear
- **THEN** each stripe has output columns that are exactly black or exactly white, and no more than one mixed column between neighbouring stripes

#### Scenario: Linear
- **WHEN** the same frame is presented at scale 2.5 with linear
- **THEN** mixed columns exist that sharp bilinear shows as pure black or white

#### Scenario: Mix unaffected
- **WHEN** a transition mix is presented with each filter at scale 1
- **THEN** the output is the same for all three
