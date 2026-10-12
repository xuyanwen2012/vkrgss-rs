## MODIFIED Requirements

### Requirement: Frame rate and frame reset
`Graphics.frame_rate` SHALL default to 40 in RGSS1 and to 60 in RGSS2, `Graphics.frame_rate=`
SHALL clamp to 10 to 120 and the value SHALL travel in the following frame messages.
`Graphics.frame_reset` SHALL send the host a frame-reset message.

#### Scenario: Frame rate in the packet
- **WHEN** `Graphics.frame_rate = 20; Graphics.update` is evaluated
- **THEN** the frame message carries frame rate 20

#### Scenario: Frame reset
- **WHEN** `Graphics.frame_reset` is evaluated
- **THEN** the host receives a frame-reset message

#### Scenario: Starting rate of VX, measured on the original
- **WHEN** the first script of a game of RGSS2 evaluates `Graphics.frame_rate` and then `Graphics.update`
- **THEN** the value is `60` and the frame message carries frame rate 60

#### Scenario: Starting rate of XP
- **WHEN** the first script of a game of RGSS1 evaluates `Graphics.frame_rate`
- **THEN** the value is `40`

## ADDED Requirements

### Requirement: Frame size by runtime
The frame of a game SHALL be 640 by 480 in RGSS1 and 544 by 416 in RGSS2, and every frame
message SHALL carry that size. In RGSS2 `Graphics.width` and `Graphics.height` SHALL return
the width and the height of the frame. In RGSS1 the two functions SHALL NOT be defined.

#### Scenario: Size of VX, measured on the original
- **WHEN** a script of a game of RGSS2 evaluates `[Graphics.width, Graphics.height]` and then `Graphics.update`
- **THEN** the result is `[544, 416]` and the frame message carries the logical size 544 by 416

#### Scenario: Size of XP
- **WHEN** a script of a game of RGSS1 evaluates `Graphics.update`
- **THEN** the frame message carries the logical size 640 by 480

#### Scenario: No such function in XP
- **WHEN** a script of a game of RGSS1 evaluates `Graphics.width`
- **THEN** `NoMethodError` is raised
