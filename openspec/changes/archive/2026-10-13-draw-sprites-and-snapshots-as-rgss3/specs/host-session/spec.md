## ADDED Requirements

### Requirement: Freeze by the rule of the message
The session SHALL ask its display to hold by the rule the freeze message names. Everything
else of "Freeze and transition" and "Transition steps of RGSS2" SHALL hold for either rule:
a freeze while frozen changes nothing, later frames are rendered and answered, and a frame of
another logical size SHALL neither end a freeze nor change the steps of a transition.

#### Scenario: The rule reaches the display
- **WHEN** a session receives a freeze message of the frame rule, a transition of duration 0 and a frame, and then a freeze message of the buffer rule
- **THEN** its display was asked to hold twice, first by the frame rule and then by the buffer rule

#### Scenario: Frozen twice
- **WHEN** two freeze messages of the buffer rule arrive with a frame between them
- **THEN** the display was asked to hold once

#### Scenario: A freeze survives a resize, measured on the original of RGSS3
- **WHEN** a frame of 544 by 416 is frozen by the buffer rule, a transition of duration 6 with the rule of RGSS2 arrives and six frames of 640 by 480 follow
- **THEN** the first five are shown at the steps 1 to 5 of 6 and the sixth alone, each answered with one tick

#### Scenario: Dump across a resize, measured on the original of RGSS3
- **WHEN** a headless run shows a frame of 544 by 416 of level 200 in every channel, freezes it by the buffer rule, receives a transition of duration 4 with the rule of RGSS2 and four frames of 640 by 480 of level 100, and dumps those four
- **THEN** the dumps are 640 by 480; pixels with x below 544 and y below 416 have the levels 175, 150, 125 and 100, and all other pixels 24, 49, 74 and 100

#### Scenario: Frame rule unchanged
- **WHEN** the tests of the session and of the headless run that existed before this change run, their freeze messages naming the frame rule
- **THEN** they pass with no expected value edited
