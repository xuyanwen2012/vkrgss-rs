# Spec Delta

## MODIFIED Requirements

### Requirement: Freeze and transition
After a freeze message the host SHALL keep showing the frame that was on screen when it arrived,
while still rendering and answering the frames that follow. After a transition message of
duration N the host SHALL show a transition from the held frame to the newest frame that
advances by one of N equal steps with each following frame, and SHALL show frames normally once
the Nth step is reached. Without a mask the transition SHALL be a cross-fade; with a mask
bitmap it SHALL be the presenter's masked mix with that bitmap and the message's vagueness. A
duration of 0 SHALL show the newest frame at once.

#### Scenario: Frozen display
- **WHEN** a freeze is followed by three frames with different contents
- **THEN** the displayed image stays the one from before the freeze and three ticks are sent

#### Scenario: Cross-fade
- **WHEN** a transition of duration 4 follows a freeze and four more frames arrive
- **THEN** after the second of them the display is the even mix of the held and the newest frame,
  and after the fourth it is the newest frame alone

#### Scenario: Transition without freeze
- **WHEN** a transition arrives while nothing is frozen
- **THEN** frames are shown normally

#### Scenario: Masked transition
- **WHEN** a transition of duration 4 names a mask bitmap that is black on the left half and white on the right half, and two more frames arrive
- **THEN** the left half of the display shows the newest frame and the right half the held frame

#### Scenario: Unknown mask
- **WHEN** a transition names a bitmap the host does not have
- **THEN** it is shown as a cross-fade and one report is made
