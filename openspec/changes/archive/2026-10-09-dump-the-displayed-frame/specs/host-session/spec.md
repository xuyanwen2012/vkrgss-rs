# Spec Delta

## MODIFIED Requirements

### Requirement: Freeze and transition
After a freeze message the host SHALL keep showing the frame then on screen while still
rendering and answering the frames that follow. After a transition message of duration N the
k-th following frame SHALL be shown at step k-1 of N equal steps from the held frame to the
newest, and frames SHALL be shown normally from the frame after the N-th. The transition SHALL
be a cross-fade, or with a mask bitmap the presenter's masked mix. When nothing is frozen, the
picture on screen SHALL stay for those N frames.

#### Scenario: Frozen display
- **WHEN** a freeze is followed by three frames with different contents
- **THEN** the displayed image stays the one from before the freeze and three ticks are sent

#### Scenario: Cross-fade
- **WHEN** a transition of duration 4 follows a freeze of a white frame and five black frames arrive
- **THEN** the display is white after the first, the even mix after the third, one quarter white after the fourth and black after the fifth

#### Scenario: Transition without freeze
- **WHEN** a white frame is on screen, nothing is frozen, a transition of duration 4 arrives and five black frames follow
- **THEN** the display stays white for the first four and is black after the fifth

#### Scenario: Masked transition
- **WHEN** a transition of duration 4 names a mask bitmap that is black on the left half and white on the right half, and three more frames arrive
- **THEN** the left half of the display shows the newest frame and the right half the held frame

#### Scenario: Unknown mask
- **WHEN** a transition names a bitmap the host does not have
- **THEN** it is shown as a cross-fade and one report is made

#### Scenario: Duration one
- **WHEN** a transition of duration 1 follows a freeze and two frames arrive
- **THEN** the display shows the held frame after the first and the newest frame after the second

#### Scenario: Duration zero
- **WHEN** a white frame is frozen, a transition of duration 0 arrives and one black frame follows
- **THEN** the display is white until that frame and black after it

## ADDED Requirements

### Requirement: Dumps show the display
A frame dump of a headless run SHALL be the image a window would show for that frame at the
frame's own size: the held frame while frozen and the mix during a transition, not the newest
composited frame.

#### Scenario: Frozen
- **WHEN** a white frame is frozen, a black frame follows and that frame is dumped
- **THEN** the dump is white

#### Scenario: Mid transition
- **WHEN** a white frame is frozen, a transition of duration 4 arrives, black frames follow and the third of them is dumped
- **THEN** the dump is the even mix of white and black

#### Scenario: No transition
- **WHEN** a run without freezes dumps a frame
- **THEN** the dump equals the composited frame, as before
