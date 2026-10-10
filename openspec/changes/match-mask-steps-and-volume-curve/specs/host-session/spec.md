# Spec Delta

## MODIFIED Requirements

### Requirement: Transition step values
For the k-th frame of a transition of duration N, shown at step `s = k - 1`, the session SHALL
give the presenter the cross-fade weight `an = 255 * s / N` and, for a masked transition, the
threshold `T = 256 * s / N - 1` in integer division, floored at 0. At step 0 the display SHALL
show the held frame alone, with or without a mask.

#### Scenario: Last step
- **WHEN** a transition of duration 8 shows its eighth frame
- **THEN** the weight is 223, and for durations 3 and 5 the thresholds of the last step are 169 and 203

#### Scenario: Intermediate thresholds
- **WHEN** a masked transition of duration 20 shows its frames
- **THEN** the thresholds of steps 1, 2, 6 and 11 are 11, 24, 75 and 139

#### Scenario: First step of a masked transition
- **WHEN** a masked transition of duration 5 with vagueness 40 shows its first frame
- **THEN** the display shows the held frame at every pixel, whatever the mask
