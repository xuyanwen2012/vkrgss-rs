## ADDED Requirements

### Requirement: Transition steps of RGSS2
After a transition message of duration N with the rule of RGSS2, the k-th following frame
SHALL be shown at step k: for k below N as the mix of the held frame and the newest with the
weight `an = 255 * k / N`, or with a mask the masked mix at the threshold of "Transition step
values" for `s = k`; the N-th frame SHALL show the newest frame alone and end the transition.
With the rule of RGSS1 the steps SHALL stay as "Freeze and transition" has them.

#### Scenario: Cross-fade, measured on the original runtime of VX
- **WHEN** a transition of duration 4 with the rule of RGSS2 follows a freeze of a white frame and four black frames arrive
- **THEN** the display shows the levels 192, 128 and 64 after the first three and black after the fourth

#### Scenario: The same frames under the rule of RGSS1
- **WHEN** the same messages are sent with the rule of RGSS1
- **THEN** the display is white after the first frame, as before this change

#### Scenario: Duration one
- **WHEN** a transition of duration 1 with the rule of RGSS2 follows a freeze and one frame arrives
- **THEN** the display shows that frame alone

#### Scenario: Held frame keeps its brightness
- **WHEN** a frame of level 200 at brightness 100 is frozen and frames at brightness 255 follow without a transition
- **THEN** the display stays at level 78
