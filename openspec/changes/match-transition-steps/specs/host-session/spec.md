# Spec Delta

## MODIFIED Requirements

### Requirement: Freeze and transition
After a freeze message the host SHALL keep showing the frame then on screen while still
rendering and answering the frames that follow. After a transition message of duration N the
k-th following frame SHALL be shown at step k-1 of N equal steps from the held frame to the
newest, and frames SHALL be shown normally from the frame after the N-th. The transition SHALL
be a cross-fade, or with a mask bitmap the presenter's masked mix with that bitmap and the
message's vagueness. A duration of 0 SHALL show the newest frame at once.

#### Scenario: Frozen display
- **WHEN** a freeze is followed by three frames with different contents
- **THEN** the displayed image stays the one from before the freeze and three ticks are sent

#### Scenario: Cross-fade
- **WHEN** a transition of duration 4 follows a freeze of a white frame and five black frames arrive
- **THEN** the display is white after the first, the even mix after the third, one quarter white after the fourth and black after the fifth

#### Scenario: Transition without freeze
- **WHEN** a transition arrives while nothing is frozen
- **THEN** frames are shown normally

#### Scenario: Masked transition
- **WHEN** a transition of duration 4 names a mask bitmap that is black on the left half and white on the right half, and three more frames arrive
- **THEN** the left half of the display shows the newest frame and the right half the held frame

#### Scenario: Unknown mask
- **WHEN** a transition names a bitmap the host does not have
- **THEN** it is shown as a cross-fade and one report is made

#### Scenario: Duration one
- **WHEN** a transition of duration 1 follows a freeze and two frames arrive
- **THEN** the display shows the held frame after the first and the newest frame after the second
