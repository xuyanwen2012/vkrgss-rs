# rgss-modules Specification

## Purpose
Defines the Ruby modules `Graphics`, `Input` and `Audio` as scripts see them, and how each call
turns into protocol messages between the guest and the host.

## Requirements

### Requirement: Method surface of the modules
`Graphics`, `Input` and `Audio` SHALL define every module function the original runtime defines
for them with the same arity, as listed in the committed method list, and `Input` SHALL define
the twenty button constants with the original's numeric values.

#### Scenario: Every listed function exists
- **WHEN** each entry of the method list for the three modules is checked with `method`
- **THEN** the function exists and its arity equals the listed arity

#### Scenario: Button constants
- **WHEN** `[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9]` is evaluated
- **THEN** the result is `[2, 4, 6, 8, 11, 13, 29]`

### Requirement: Graphics.update
`Graphics.update` SHALL extract a frame from the scene, send it to the host as one frame
message, and return when the host's tick for that frame has arrived. Host messages received
while waiting SHALL be applied: an input snapshot is kept for the next `Input.update`, a reset
request makes this call raise `Reset`, and a quit request makes it raise `SystemExit`.

#### Scenario: One frame per call
- **WHEN** a script calls `Graphics.update` three times against a host that ticks each frame
- **THEN** the host has received exactly three frame messages

#### Scenario: Frame count
- **WHEN** `Graphics.frame_count = 0` is followed by five calls of `Graphics.update`
- **THEN** `Graphics.frame_count` is `5`

#### Scenario: Quit
- **WHEN** the host sends a quit request while a script is in `Graphics.update`
- **THEN** `SystemExit` is raised in the script

### Requirement: Frame rate and frame reset
`Graphics.frame_rate` SHALL default to 40, `Graphics.frame_rate=` SHALL clamp to 10 to 120 and
the value SHALL travel in the following frame messages. `Graphics.frame_reset` SHALL send the
host a frame-reset message.

#### Scenario: Frame rate in the packet
- **WHEN** `Graphics.frame_rate = 20; Graphics.update` is evaluated
- **THEN** the frame message carries frame rate 20

#### Scenario: Frame reset
- **WHEN** `Graphics.frame_reset` is evaluated
- **THEN** the host receives a frame-reset message

### Requirement: Freeze and transition
`Graphics.freeze` SHALL send the host a freeze message. `Graphics.transition(duration = 8,
filename = nil, vague = 20)` SHALL send a transition message with that duration, the bitmap
loaded from `filename` as mask when one is given, and the vagueness, and SHALL then send one
frame and wait for its tick `duration` times before returning. A filename that is not found
SHALL raise `Errno::ENOENT`.

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

### Requirement: Input
`Input.update` SHALL latch the most recent input snapshot received from the host and advance
the state that `press?`, `trigger?`, `repeat?`, `dir4` and `dir8` report, as `core-input`
defines. A button number that is not one of the twenty constants SHALL be reported as not
pressed.

#### Scenario: Trigger
- **WHEN** the host sends a snapshot with C held, and the script calls `Input.update` twice
- **THEN** `Input.trigger?(Input::C)` is `true` after the first call and `false` after the second, and `Input.press?(Input::C)` is `true` after both

#### Scenario: Unknown button
- **WHEN** `Input.press?(99)` is evaluated
- **THEN** the result is `false`

### Requirement: Audio playback
`Audio.bgm_play`, `bgs_play`, `me_play` and `se_play` SHALL take a filename, a volume
(default 100, clamped to 0 to 100) and a pitch (default 100, clamped to 50 to 150), find the
file as audio through the game's files, and send the host a play command for the matching
channel with the file's contents. A file that is not found SHALL raise `Errno::ENOENT` with
the filename as given.

#### Scenario: Play
- **WHEN** `Audio.se_play("Audio/SE/001-System01", 80)` is evaluated and `Audio/SE/001-System01.ogg` exists in the run-time package
- **THEN** the host receives a play command for the sound-effect channel with volume 80, pitch 100 and the bytes of that file

#### Scenario: Missing file
- **WHEN** `Audio.bgm_play("Audio/BGM/Nope")` is evaluated
- **THEN** `Errno::ENOENT` is raised with the message `No such file or directory - Audio/BGM/Nope`

#### Scenario: Same music again
- **WHEN** `Audio.bgm_play` is called twice in a row with the same filename, volume and pitch
- **THEN** only one play command is sent

### Requirement: Audio stop and fade
The `stop` functions SHALL send the host a stop command for their channel, and the `fade`
functions a fade command with the given time in milliseconds.

#### Scenario: Stop and fade
- **WHEN** `Audio.bgm_stop; Audio.bgm_fade(800)` is evaluated
- **THEN** the host receives a stop command and a fade command of 800 milliseconds for the music channel
