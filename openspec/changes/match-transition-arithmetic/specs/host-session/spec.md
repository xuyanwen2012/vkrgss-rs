# Spec Delta

## ADDED Requirements

### Requirement: Transition step values
For the k-th frame of a transition of duration N, shown at step `s = k - 1`, the session SHALL
give the presenter the cross-fade weight `an = 255 * s / N` and, for a masked transition, the
threshold `T = (255 * s - 1) / N` in integer division, floored at 0.

#### Scenario: Last step
- **WHEN** a transition of duration 8 shows its eighth frame
- **THEN** the weight is 223, and for durations 3 and 5 the thresholds of the last step are 169 and 203

### Requirement: Mask bitmaps reach the display
A transition picture that the guest loaded SHALL be available to the display when the first
frame of the transition is shown, so that a mask that exists is never replaced by a
cross-fade.

#### Scenario: Mask file of a game
- **WHEN** a generated game with a picture in `Graphics/Transitions` runs `Graphics.freeze`, draws, and calls `Graphics.transition(10, "Graphics/Transitions/<name>")` headless
- **THEN** no report about a missing mask is printed and the dumped frame inside the transition shows the mask's pattern
