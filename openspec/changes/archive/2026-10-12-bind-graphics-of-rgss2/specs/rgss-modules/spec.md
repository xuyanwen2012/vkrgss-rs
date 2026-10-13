## MODIFIED Requirements

### Requirement: Freeze and transition
`Graphics.freeze` SHALL send the host a freeze message. In RGSS1
`Graphics.transition(duration = 8, filename = nil, vague = 20)` SHALL send a transition
message with that duration, the bitmap loaded from `filename` as mask when one is given, and
the vagueness, and SHALL then send one frame and wait for its tick `duration` times before
returning. A filename that is not found SHALL raise `Errno::ENOENT`. In RGSS2 the function
SHALL follow "Transition of RGSS2".

#### Scenario: Default transition
- **WHEN** `Graphics.freeze; Graphics.transition` is evaluated
- **THEN** the host receives a freeze message, a transition message with duration 8 and no mask, and then eight frame messages

#### Scenario: Zero duration
- **WHEN** `Graphics.transition(0)` is evaluated
- **THEN** a transition message with duration 0 is sent and no frame message

#### Scenario: Transition picture
- **WHEN** `Graphics.transition(40, "Graphics/Transitions/004-Blind04")` is evaluated and the picture exists
- **THEN** the transition message names a mask bitmap, and the first frame message that follows carries the creation of that bitmap

#### Scenario: Default vagueness
- **WHEN** `Graphics.transition(5, "Graphics/Transitions/004-Blind04")` is evaluated and the picture exists
- **THEN** the transition message carries the vagueness 20

#### Scenario: Rule of XP
- **WHEN** a game of RGSS1 evaluates `Graphics.freeze; Graphics.transition(4)`
- **THEN** the transition message carries the rule of RGSS1

## ADDED Requirements

### Requirement: Transition of RGSS2
In RGSS2, `Graphics.transition(duration = 10, ...)` with nothing frozen SHALL send nothing,
show no frame and leave the brightness. After a freeze it SHALL set the brightness to 255,
send a transition message with the rule of RGSS2, and then send `duration` frames.

#### Scenario: Default, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.freeze; Graphics.transition`
- **THEN** the host receives a freeze message, a transition message with duration 10 and the rule of RGSS2, and ten frame messages

#### Scenario: Nothing frozen, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.brightness = 100; Graphics.transition(20)` with nothing frozen
- **THEN** the host receives no transition message and no frame message, `Graphics.frame_count` is unchanged and `Graphics.brightness` is `100`

#### Scenario: Brightness after a freeze, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.fadeout(1); Graphics.freeze; Graphics.transition(0); Graphics.brightness`
- **THEN** the value is `255` and no frame message follows the transition message

### Requirement: Brightness of RGSS2
In RGSS2 `Graphics.brightness` SHALL return the brightness of the scene, 255 at the start, and
`Graphics.brightness=` SHALL set it, clamped to 0 to 255; the following frame messages SHALL
carry it. In RGSS1 neither function SHALL be defined.

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.brightness = -5` and then `Graphics.brightness = 300`, reading the value after each
- **THEN** the values are `0` and `255`

#### Scenario: In the packet
- **WHEN** a game of RGSS2 evaluates `Graphics.brightness = 100; Graphics.update`
- **THEN** the frame message carries the brightness 100

#### Scenario: Not in XP
- **WHEN** a game of RGSS1 evaluates `Graphics.brightness`
- **THEN** `NoMethodError` is raised

### Requirement: Waits and fades of RGSS2
In RGSS2 `Graphics.wait(n)` SHALL send n frames as `Graphics.update` does, none for n below 1.
`Graphics.fadeout(n)` and `Graphics.fadein(n)` SHALL send n frames, the k-th at the
brightness `b0 - b0 * k / n` and `b0 + (255 - b0) * k / n` in integer division, where b0 is
the brightness before the call, and SHALL leave 0 and 255 also when n is 0. None of them SHALL
call the Ruby method `Graphics.update`. In RGSS1 they SHALL NOT be defined.

#### Scenario: Wait, measured on the original
- **WHEN** a game of RGSS2 sets `Graphics.frame_count = 0` and evaluates `Graphics.wait(7); Graphics.wait(0); Graphics.wait(-3)`
- **THEN** the host has received seven frame messages and `Graphics.frame_count` is `7`

#### Scenario: Fade out, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.brightness = 200; Graphics.fadeout(3)`
- **THEN** the three frame messages carry the brightness 134, 67 and 0, and `Graphics.brightness` is `0`

#### Scenario: Fade in, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.brightness = 100; Graphics.fadein(4)`
- **THEN** the four frame messages carry the brightness 138, 177, 216 and 255

#### Scenario: No frame
- **WHEN** a game of RGSS2 evaluates `Graphics.fadeout(0)` and then `Graphics.fadein(0)`, reading the brightness after each
- **THEN** no frame message is sent and the values are `0` and `255`

#### Scenario: A redefined update is not called
- **WHEN** a script of a game of RGSS2 redefines `Graphics.update` to count its calls and evaluates `Graphics.wait(5); Graphics.fadeout(5)`
- **THEN** the count is `0` and ten frame messages were sent

### Requirement: Snapshot of the scene
In RGSS2 `Graphics.snap_to_bitmap` SHALL return a new bitmap of the size of the frame that
holds the scene as its objects are at that moment, also while the display is frozen, darkened
by the brightness as a frame is, with alpha 255 at every pixel. It SHALL send no frame and
SHALL NOT change the frame count or what later frame messages carry. In RGSS1 it SHALL NOT be
defined.

#### Scenario: Brightness, measured on the original
- **WHEN** a game of RGSS2 shows a sprite of level 200 over the whole frame, sets `Graphics.brightness = 100` and reads a pixel of `Graphics.snap_to_bitmap`
- **THEN** the pixel is `Color.new(78, 78, 78, 255)` and the bitmap is 544 by 416

#### Scenario: Changes not yet shown and a freeze, measured on the original
- **WHEN** a game of RGSS2 shows a red sprite, calls `Graphics.update` and `Graphics.freeze`, makes the sprite's bitmap blue and takes a snapshot without an update
- **THEN** the snapshot is blue where the sprite is

#### Scenario: Nothing owed to the host is lost
- **WHEN** a game of RGSS2 changes a bitmap, takes a snapshot and calls `Graphics.update`
- **THEN** `Graphics.frame_count` rose by one, and the frame message carries the change of the bitmap and the frame the host composes from it equals the frame of the same script without the snapshot

### Requirement: Frames of a game of RGSS2 as shown
A game of RGSS2 run by the program SHALL show the frames of a fade and of a transition after a
freeze as the original does.

#### Scenario: Dumped frames of a game
- **WHEN** a generated game of RGSS2 with a white sprite over the whole frame runs headless: `Graphics.update`, `Graphics.fadeout(4)`, `Graphics.freeze`, `Graphics.transition(4)`, `Graphics.update`, and frames 1 to 10 are dumped
- **THEN** the levels of the frames are 255; 191, 127, 63, 0; 62, 126, 190, 255; 255
