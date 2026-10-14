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

### Requirement: Resizing the screen
In RGSS2 `Graphics.resize_screen(width, height)` SHALL set the size of the frame to the two
integers, the width limited to 1 to 640 and the height to 1 to 480, SHALL return `nil` and
SHALL send no frame. The following frame messages, `Graphics.width`, `Graphics.height` and
`Graphics.snap_to_bitmap` SHALL have the new size. In RGSS1 the function SHALL NOT be defined.

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(w, h); [Graphics.width, Graphics.height]` for (641, 481), (1000, 1000), (0, 0), (-1, -1), (100, 50) and (640, 1)
- **THEN** the results are `[640, 480]`, `[640, 480]`, `[1, 1]`, `[1, 1]`, `[100, 50]` and `[640, 1]`

#### Scenario: Frames and snapshot, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(320, 240)`, takes a snapshot and calls `Graphics.update`
- **THEN** the snapshot is 320 by 240, the frame count rose by one, and the frame message carries the logical size 320 by 240

#### Scenario: Arguments, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(640)`, `Graphics.resize_screen(nil, 480)` and `Graphics.resize_screen(100.9, 50.9)`
- **THEN** the first raises `ArgumentError`, the second `TypeError`, and after the third the size is 100 by 50

#### Scenario: Dump after a resize
- **WHEN** a generated game of RGSS2 with a red sprite at (600, 440) calls `Graphics.update`, `Graphics.resize_screen(640, 480)` and `Graphics.update`, headless with both frames dumped
- **THEN** the first picture is 544 by 416 without red and the second 640 by 480 with red at (600, 440)

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Graphics.resize_screen(640, 480)`
- **THEN** `NoMethodError` is raised

### Requirement: Small rules of RGSS2 in the modules and elements
In RGSS2 the vagueness of `Graphics.transition` SHALL default to 40, and `blend_type=` of a
sprite and of a plane SHALL raise `ArgumentError` with the message `bad value for blend_type`
for a value that, truncated, is not 0, 1 or 2. In RGSS1 both SHALL stay as they are.

#### Scenario: Default vagueness, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.freeze; Graphics.transition(5, "Graphics/System/Mask")` and the picture exists
- **THEN** the transition message carries the vagueness 40

#### Scenario: Blend type, measured on the original
- **WHEN** a game of RGSS2 evaluates `s.blend_type = 1.9; s.blend_type` and then `s.blend_type = 3`
- **THEN** the value is `1` and the assignment raises `ArgumentError`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `s.blend_type = 3`
- **THEN** no error is raised

### Requirement: Reset functions of RGSS3
In a game of RGSS3 `Graphics.__reset__` and `Audio.__reset__` SHALL take no argument and
return `nil`. The first SHALL dispose every `Bitmap`, `Sprite`, `Viewport`, `Window`,
`Plane` and `Tilemap`, set the brightness to 255 and the frame rate to 60 and end a freeze,
and SHALL leave the frame count, the size of the screen, the values of `Font` and the audio
as they are. The second SHALL stop the audio of all four channels.

#### Scenario: Arity and value, measured on the original
- **WHEN** a script of a game of RGSS3 evaluates `[Graphics.method(:__reset__).arity, Graphics.__reset__, Audio.method(:__reset__).arity, Audio.__reset__]`
- **THEN** the result is `[0, nil, 0, nil]`

#### Scenario: What is disposed, measured on the original
- **WHEN** a script makes a free `Bitmap`, a `Sprite` of a subclass with a bitmap, a `Viewport`, a `Window` with a skin and contents, a `Plane` and a `Tilemap`, calls `Graphics.__reset__` and asks each for `disposed?`
- **THEN** every answer is `true`, the sprite still returns its bitmap, and `sprite.x` raises `RGSSError` with the message `disposed sprite`

#### Scenario: What is set back, measured on the original
- **WHEN** a script sets `Graphics.brightness = 100` and `Graphics.frame_rate = 120`, calls `Graphics.freeze` and then `Graphics.__reset__`
- **THEN** `Graphics.brightness` is 255 and `Graphics.frame_rate` is 60, and the display is not frozen: the frame of the next `Graphics.update` is shown

#### Scenario: What stays, measured on the original
- **WHEN** a script sets `Graphics.frame_count = 1000`, calls `Graphics.resize_screen(320, 240)`, sets `Font.default_size = 30`, keeps a `Color` read from a sprite and calls `Graphics.__reset__`
- **THEN** `Graphics.frame_count` is 1000, `[Graphics.width, Graphics.height]` is `[320, 240]`, `Font.default_size` is 30 and the `Color` has its values

#### Scenario: Audio, measured on the original
- **WHEN** a script plays a music, a background sound, a music effect and a sound effect and calls `Audio.__reset__`
- **THEN** the host receives a stop for each of the four channels, and `Graphics.__reset__` alone sends none

#### Scenario: Redefined by a script, measured on the original
- **WHEN** a script redefines `Audio.__reset__` and `Graphics.__reset__` to append their names to a global array and the block of `rgss_main` of the stand-in raises `RGSSReset` once
- **THEN** the array is `["Audio", "Graphics"]`

#### Scenario: Games of RGSS1 and RGSS2
- **WHEN** a script of a game of RGSS1 or of RGSS2 evaluates `Graphics.respond_to?(:__reset__)` over Ruby 1.8.7
- **THEN** the result is `false`

### Requirement: Method surface of Graphics in RGSS3
In a game of RGSS3 the public module functions of `Graphics` SHALL be exactly those of the
committed method list of RGSS3, with their arities: `play_movie` SHALL be defined, and
`_reset` SHALL NOT be callable from a script although the guest still uses it.

#### Scenario: The functions, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.singleton_methods(false).sort`
- **THEN** the result is `[:__reset__, :brightness, :brightness=, :fadein, :fadeout, :frame_count, :frame_count=, :frame_rate, :frame_rate=, :frame_reset, :freeze, :height, :play_movie, :resize_screen, :snap_to_bitmap, :transition, :update, :visible, :visible=, :wait, :width]`

#### Scenario: No _reset for scripts
- **WHEN** a game of RGSS3 evaluates `Graphics.respond_to?(:_reset)` and then `Graphics.__reset__`
- **THEN** the first is `false` and the second returns `nil`

#### Scenario: XP and VX
- **WHEN** a game of RGSS2 evaluates `[Graphics.respond_to?(:_reset), Graphics.respond_to?(:play_movie)]`
- **THEN** the result is `[true, false]`

### Requirement: Freeze and snapshot of RGSS3
In a game of RGSS3 `Graphics.freeze` SHALL send a freeze message with the buffer rule, and
`Graphics.snap_to_bitmap` SHALL make its pixels by the snapshot rule of RGSS3: as in RGSS2,
and every byte 0 when the brightness is 0. It SHALL take no argument and show no frame.

#### Scenario: Freeze rule
- **WHEN** a game of RGSS3 evaluates `Graphics.freeze; Graphics.transition(2)`
- **THEN** the host receives a freeze message with the buffer rule, a transition message of 2 frames with the rule of RGSS2, and two frames

#### Scenario: Brightness 0, measured on the original
- **WHEN** a game of RGSS3 shows an opaque white sprite over the whole screen, sets `Graphics.brightness = 0` and reads the pixel (10, 10) of `Graphics.snap_to_bitmap` as `[red, green, blue, alpha]`
- **THEN** it is `[0.0, 0.0, 0.0, 0.0]`, the bitmap is 544 by 416, and no frame was sent

#### Scenario: Other brightness, measured on the original
- **WHEN** the same is read at `Graphics.brightness = 100`, and at 255 with nothing drawn
- **THEN** the pixels are `[99.0, 99.0, 99.0, 255.0]` and `[0.0, 0.0, 0.0, 255.0]`

#### Scenario: Argument, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.snap_to_bitmap(1)`
- **THEN** an `ArgumentError` is raised

### Requirement: Transition of RGSS3
In a game of RGSS3 `Graphics.transition` with nothing frozen SHALL return `nil` without
reading its arguments. After a freeze it SHALL follow "Transition of RGSS2", count its
arguments with the measured message, raise `TypeError` for a name that is no String, take
an empty name as none, and end the freeze when the picture of a name is not found; any
other error SHALL keep the freeze.

#### Scenario: Nothing frozen, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.transition`, `Graphics.transition(5, "missing")`, `Graphics.transition(1, 2, 3, 4)` and `Graphics.transition(nil)` with nothing frozen
- **THEN** each returns `nil`, and the host receives no transition message and no frame

#### Scenario: Durations, measured on the original
- **WHEN** a game of RGSS3 evaluates, each after `Graphics.freeze`, `transition`, `transition(3)`, `transition(2.9)`, `transition(-3)` and `transition(0)`, and after the last two another `transition(3)`
- **THEN** they show 10, 3, 2, 0 and 0 frames, the two later `transition(3)` none, and the brightness is 255 after each

#### Scenario: Errors that keep the freeze, measured on the original
- **WHEN** a game of RGSS3 evaluates, each after `Graphics.freeze` and followed by `transition(3)`, `transition(nil)`, `transition(3, nil)`, `transition(3, 5)` and `transition(3, "", 10, 4)`
- **THEN** they raise `TypeError` with `no implicit conversion from nil to integer`, `TypeError` with `can't convert nil into String`, `TypeError` with `can't convert Fixnum into String` and `ArgumentError` with `wrong number of arguments (3 for 4)`, none shows a frame, and each following `transition(3)` shows 3 frames

#### Scenario: A missing picture ends the freeze, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.freeze`, then `Graphics.transition(3, "missing")`, then `Graphics.transition(3)`
- **THEN** the first transition raises `Errno::ENOENT` with `No such file or directory - missing` and shows no frame, and the second shows no frame

#### Scenario: Empty name, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.freeze; Graphics.transition(3, "")`
- **THEN** no error is raised, the transition message has no mask, and 3 frames are shown

#### Scenario: Two freezes, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.freeze; Graphics.freeze; Graphics.transition(2); Graphics.transition(2)`
- **THEN** 2 frames are shown in all

### Requirement: Movies of RGSS3
In a game of RGSS3 `Graphics.play_movie(name)` SHALL take one String, raise `Errno::ENOENT`
naming the name as given when no file of the game directory is found under it or under it
with `.ogv` appended, and otherwise return `nil`. It SHALL show no frame in either case and
SHALL play nothing.

#### Scenario: Errors, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.play_movie`, `Graphics.play_movie(nil)`, `Graphics.play_movie(5)`, `Graphics.play_movie("Movies/missing")` and `Graphics.play_movie("")`
- **THEN** they raise `ArgumentError`, `TypeError` with `can't convert nil into String`, `TypeError` with `can't convert Fixnum into String`, `Errno::ENOENT` with `No such file or directory - Movies/missing` and `Errno::ENOENT` with `No such file or directory - `, and the host receives no frame

#### Scenario: A file that exists
- **WHEN** the game directory holds `Movies/clip.ogv` and a game of RGSS3 evaluates `Graphics.play_movie("Movies/clip")` and `Graphics.play_movie("Movies/clip.ogv")`
- **THEN** both return `nil` and the host receives no frame and no other message

### Requirement: Graphics values of RGSS3
In a game of RGSS3 `wait`, `fadeout`, `fadein`, `brightness`, `frame_rate`, `frame_count`,
`width`, `height`, `visible` and `resize_screen` SHALL behave as in RGSS2, with the
arguments converted as measured. `resize_screen` SHALL change no viewport and keep the
brightness, the frame count, the frame rate and a freeze.

#### Scenario: Waits and fades, measured on the original
- **WHEN** a game of RGSS3 evaluates `wait(3)`, `wait(2.9)`, `wait(-2)`, `fadeout(2.9)`, `fadein(2.9)`, `fadeout(-3)`, `fadein(-3)`, and `brightness = 100; fadeout(4)`, counting frames and reading the brightness after each
- **THEN** the frames are 3, 2, 0, 2, 2, 0, 0 and 4, and the brightness after the fades is 0, 255, 0, 255 and 0

#### Scenario: Wrong arguments of waits and fades, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.wait(nil)`, `Graphics.wait("2")` and `Graphics.fadeout(nil)`
- **THEN** they raise `TypeError` with `no implicit conversion from nil to integer`, `can't convert String into Integer` and `no implicit conversion from nil to integer`, and no frame is shown

#### Scenario: Brightness and frame rate, measured on the original
- **WHEN** a game of RGSS3 sets and reads back `brightness` to 300, -5 and 99.9, and `frame_rate` to 9, 121, 59.9 and -1, and evaluates `Graphics.brightness = nil` and `Graphics.frame_rate = nil`
- **THEN** the values read are 255, 0, 99, 10, 120, 59 and 10, and both assignments of `nil` raise `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Start values, measured on the original
- **WHEN** a game of RGSS3 reads `[Graphics.width, Graphics.height, Graphics.frame_rate, Graphics.brightness, Graphics.visible]` at its start
- **THEN** the result is `[544, 416, 60, 255, true]`

#### Scenario: Frame count, measured on the original
- **WHEN** a game of RGSS3 sets and reads back `Graphics.frame_count` to -5 and 7.9
- **THEN** the values read are -5 and 7

#### Scenario: Resize limits, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.resize_screen(w, h); [Graphics.width, Graphics.height]` for (800, 600), (641, 481), (300, 500), (0, 0), (-5, 10) and (200.9, 100.9)
- **THEN** the results are `[640, 480]`, `[640, 480]`, `[300, 480]`, `[1, 1]`, `[1, 10]` and `[200, 100]`, each call returns `nil` and no frame is shown

#### Scenario: Resize arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Graphics.resize_screen(100)` and `Graphics.resize_screen(nil, 5)`
- **THEN** the first raises `ArgumentError` and the second `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: What a resize keeps, measured on the original
- **WHEN** a game of RGSS3 sets the brightness 100, the frame count 7 and the frame rate 30, calls `Graphics.freeze` and `Graphics.resize_screen(640, 480)`, reads the three values and calls `Graphics.transition(6)`
- **THEN** the values are 100, 7 and 30, and the transition shows 6 frames of 640 by 480
