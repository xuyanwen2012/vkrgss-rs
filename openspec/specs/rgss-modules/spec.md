# rgss-modules Specification

## Purpose
Defines the Ruby modules `Graphics`, `Input` and `Audio` as scripts see them, and how each call
turns into protocol messages between the guest and the host.

## Requirements

### Requirement: Method surface of the modules
`Graphics`, `Input` and `Audio` SHALL define every module function the original runtime defines
for them with the same arity, as listed in the committed method list, and `Input` SHALL define
the twenty button constants with the original's numeric values in RGSS1 and RGSS2, and with
the Symbols of their names in RGSS3.

#### Scenario: Every listed function exists
- **WHEN** each entry of the method list for the three modules is checked with `method`
- **THEN** the function exists and its arity equals the listed arity

#### Scenario: Button constants
- **WHEN** `[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9]` is evaluated
- **THEN** the result is `[2, 4, 6, 8, 11, 13, 29]`

#### Scenario: Button constants of RGSS3, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9, Input::DOWN == :DOWN]`
- **THEN** the result is `[:DOWN, :LEFT, :RIGHT, :UP, :A, :C, :F9, true]`

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
In RGSS1 and RGSS2 `Input.update` SHALL latch the most recent input snapshot received from
the host and advance the state that `press?`, `trigger?`, `repeat?`, `dir4` and `dir8`
report, as `core-input` defines. A button number that is not one of the twenty constants
SHALL be reported as not pressed. In RGSS3 `Input` SHALL follow the requirements of `Input`
of RGSS3.

#### Scenario: Trigger
- **WHEN** the host sends a snapshot with C held, and the script calls `Input.update` twice
- **THEN** `Input.trigger?(Input::C)` is `true` after the first call and `false` after the second, and `Input.press?(Input::C)` is `true` after both

#### Scenario: Unknown button
- **WHEN** `Input.press?(99)` is evaluated
- **THEN** the result is `false`

### Requirement: Audio playback
In RGSS1 and RGSS2 `Audio.bgm_play`, `bgs_play`, `me_play` and `se_play` SHALL take a
filename, a volume (default 100, clamped to 0 to 100) and a pitch (default 100, clamped to
50 to 150). In every runtime they SHALL find the file as audio through the game's files and
send the host a play command for the matching channel with the file's contents, and a file
that is not found SHALL raise `Errno::ENOENT` with the filename as given.

#### Scenario: Play
- **WHEN** `Audio.se_play("Audio/SE/001-System01", 80)` is evaluated and `Audio/SE/001-System01.ogg` exists in the run-time package
- **THEN** the host receives a play command for the sound-effect channel with volume 80, pitch 100 and the bytes of that file

#### Scenario: Missing file
- **WHEN** `Audio.bgm_play("Audio/BGM/Nope")` is evaluated
- **THEN** `Errno::ENOENT` is raised with the message `No such file or directory - Audio/BGM/Nope`

#### Scenario: Same music again
- **WHEN** `Audio.bgm_play` is called twice in a row with the same filename, volume and pitch
- **THEN** only one play command is sent

#### Scenario: Fields that RGSS3 uses
- **WHEN** a game of RGSS1 or of RGSS2 evaluates `Audio.bgm_play(name, 80)` and `Audio.me_play(name)` for a file that exists
- **THEN** both play commands carry the start 0 and the return time 0, and no volume command is sent

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

### Requirement: Functions of Audio in RGSS3
In a game of RGSS3 `Audio.bgm_play` and `Audio.bgs_play` SHALL take a filename and up to
three of volume, pitch and `pos`; `me_play` and `se_play` a filename and up to two of volume
and pitch; `Audio.bgm_pos`, `Audio.bgs_pos` and `Audio.setup_midi` SHALL take no argument.
All SHALL return `nil` except the two positions, which return an Integer. In RGSS1 and RGSS2
the three new functions SHALL NOT be defined and a fourth argument SHALL be refused.

#### Scenario: Surface, measured on the original
- **WHEN** a game of RGSS3 evaluates `(Audio.singleton_methods - [:__reset__]).sort.map { |m| [m, Audio.method(m).arity] }`
- **THEN** the result is `[[:bgm_fade, 1], [:bgm_play, -1], [:bgm_pos, 0], [:bgm_stop, 0], [:bgs_fade, 1], [:bgs_play, -1], [:bgs_pos, 0], [:bgs_stop, 0], [:me_fade, 1], [:me_play, -1], [:me_stop, 0], [:se_play, -1], [:se_stop, 0], [:setup_midi, 0]]`

#### Scenario: Four arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 80, 100, 0)` and `Audio.bgs_play(name, 80, 100, 0)` for a file that exists
- **THEN** both return `nil` and the host receives a play command for the music channel and one for the background-sound channel, each with volume 80, pitch 100 and the bytes of the file

#### Scenario: Before anything plays, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Audio.bgm_pos, Audio.bgs_pos, Audio.setup_midi, Audio.setup_midi]` before any sound was played
- **THEN** the result is `[0, 0, nil, nil]` and the host receives no audio command

#### Scenario: Counts, measured on the original
- **WHEN** a game of RGSS3 calls `Audio.bgm_play` and `Audio.bgs_play` with no argument and with five, and `Audio.me_play` and `Audio.se_play` with four
- **THEN** each raises `ArgumentError`, with the messages `wrong number of arguments (1 for 0)`, `wrong number of arguments (4 for 5)` and `wrong number of arguments (3 for 4)`

#### Scenario: Kinds of arguments, measured on the original
- **WHEN** a game of RGSS3 calls `Audio.bgm_play` for a file that exists with the volume `nil`, with the volume `"a"`, with the pitch `true`, with the `pos` `"a"` and with the `pos` `2**31`
- **THEN** the first raises `TypeError` with the message `no implicit conversion from nil to integer`, the second `TypeError` with `can't convert String into Integer`, the third `TypeError` with `can't convert true into Integer`, the fourth `TypeError` and the fifth `RangeError`

#### Scenario: Forms that are accepted, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 99.9, 100.7, nil)` and then `Audio.bgs_play(name, 100, 100, 44100.7)` for a file of 44100 Hz in stereo
- **THEN** both return `nil`, the first play command carries volume 99, pitch 100 and the start 0, and the second the start 11025

#### Scenario: Missing file while a music plays, measured on the original
- **WHEN** a game of RGSS3 plays a music, shows 60 frames, calls `Audio.bgm_play` with a name that no file has, rescues the error and shows 12 more frames
- **THEN** the error is `Errno::ENOENT` with the name as given, the host received one play command, and `Audio.bgm_pos` is greater than it was before the failed call

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 or of RGSS2 evaluates `[:bgm_pos, :bgs_pos, :setup_midi].map { |m| Audio.respond_to?(m) }` and `Audio.bgm_play(name, 100, 100, 0)` for a file that exists
- **THEN** the first is `[false, false, false]` and the second raises `ArgumentError`

### Requirement: Volume, pitch and start of a play command of RGSS3
In a game of RGSS3 a play command SHALL carry the volume 100 for a volume argument of 100 or
more or below 0, and the argument otherwise; the pitch 100 for an argument below 1 or one
that asks for more than 200000 Hz of a file whose rate is known, and the argument otherwise;
and as its start the sample frame of `pos`, 0 for `nil`, for a value below 0 and for a file
whose format is not known. The command of a music effect SHALL carry a return time of
1000 ms.

#### Scenario: Volume, measured on the original
- **WHEN** a game of RGSS3 plays a sound effect with the volumes -1000, -1, 0, 1, 99, 100, 101 and 2147483647
- **THEN** the play commands carry the volumes 100, 100, 0, 1, 99, 100, 100 and 100

#### Scenario: Pitch, measured on the original
- **WHEN** a game of RGSS3 plays a WAV file of 44100 Hz as a sound effect with the pitches -1, 0, 1, 49, 151, 200, 453, 454 and 1000, and one of 22050 Hz with 907 and 1000
- **THEN** the play commands carry the pitches 100, 100, 1, 49, 151, 200, 453, 100 and 100, and 907 and 100

#### Scenario: Start
- **WHEN** a game of RGSS3 plays a WAV file of 44100 Hz in stereo as music with the `pos` 0, 3, 4, 100001 and -44100, each after `Audio.bgm_stop`, and a WAV file of 22050 Hz in mono with the `pos` 22050
- **THEN** the play commands carry the starts 0, 0, 1, 25000 and 0, and 11025

#### Scenario: A file whose format is not known
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 100, 1000, 100000)` for a file that begins with the header of a Standard MIDI file
- **THEN** the play command carries the pitch 1000 and the start 0

#### Scenario: Return time
- **WHEN** a game of RGSS3 evaluates `Audio.me_play(name); Audio.bgm_play(name); Audio.se_play(name)`
- **THEN** the command of the music effect carries the return time 1000 and the two others the return time 0

### Requirement: The same music again in RGSS3
In a game of RGSS3 a `bgm_play` or `bgs_play` whose filename and pitch equal those of what
its channel plays SHALL send no play command and SHALL leave the position where it is,
whatever `pos` it gives; when its volume differs it SHALL send a volume command. After a
stop or a fade of the channel, and with another filename or pitch, the sound SHALL start
anew at its `pos`.

#### Scenario: Same again, measured on the original
- **WHEN** a game of RGSS3 plays a streamed music of 44100 Hz in stereo, shows 60 frames at 60 frames per second and evaluates `Audio.bgm_play(name, 100, 100, 300000); Audio.bgm_pos`
- **THEN** the host has received one play command and no volume command, and the position is 132304

#### Scenario: Another volume, measured on the original
- **WHEN** after the same 60 frames the game evaluates `Audio.bgm_play(name, 80); Audio.bgm_pos`
- **THEN** the host receives a volume command for the music channel with the volume 80 and no second play command, and the position is 132304

#### Scenario: Another pitch, measured on the original
- **WHEN** after the same 60 frames the game evaluates `Audio.bgm_play(name, 100, 150); Audio.bgm_pos`
- **THEN** the host receives a second play command with the pitch 150 and the position is 0

#### Scenario: Another spelling of the name, measured on the original
- **WHEN** after the same 60 frames the game plays the file under its name in another letter case
- **THEN** the host receives a second play command and the position is 0

#### Scenario: After a fade, measured on the original
- **WHEN** a game of RGSS3 plays a music, shows 60 frames, evaluates `Audio.bgm_fade(1000)`, shows 120 frames and evaluates `Audio.bgm_play` with the same arguments and then `Audio.bgm_pos`
- **THEN** the host receives a second play command and the position is 0

#### Scenario: Background sound
- **WHEN** a game of RGSS3 evaluates `Audio.bgs_play(name, 100); Audio.bgs_play(name, 50); Audio.bgs_play(name, 50)`
- **THEN** the host receives one play command and one volume command with the volume 50 for the background-sound channel

### Requirement: Position of the music and of the background sound
In a game of RGSS3 `Audio.bgm_pos` and `Audio.bgs_pos` SHALL return the position of the
model of what their channel plays, in bytes of 16-bit PCM: 0 before the first play and for a
sound whose format is not known, the last value after a stop, and a value that goes on
during a fade. The guest SHALL compute it from the frames it has shown since the play and
SHALL NOT ask the host, so that it is the same with any audio sink and in every run with
the same input.

#### Scenario: At once, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 0, 100, p); Audio.bgm_pos` for a WAV file of 20 s at 44100 Hz in stereo and `p` of 0, 1, 44100, 100000 and 100001, each after `Audio.bgm_stop`
- **THEN** the positions are 0, 0, 11024, 66924 and 66925

#### Scenario: Every sixth frame, measured on the original
- **WHEN** a game of RGSS3 at 60 frames per second plays that file as music and reads `Audio.bgm_pos` at once and after every six calls of `Graphics.update`, fifteen readings in all
- **THEN** the readings are 0, 0, 0, 0, 33076, 33076, 66152, 66152, 99228, 99228, 132304, 132304, 165380, 165380 and 198456

#### Scenario: Background sound and both together, measured on the original
- **WHEN** a game of RGSS3 plays that file as music and as background sound, shows 60 frames and evaluates `[Audio.bgm_pos, Audio.bgs_pos]`
- **THEN** the result is `[132304, 132304]`

#### Scenario: Frames of a wait
- **WHEN** a game of RGSS3 plays that file as music and evaluates `Graphics.wait(60); Audio.bgm_pos`
- **THEN** the position is 132304

#### Scenario: No frame, no step
- **WHEN** a game of RGSS3 plays that file as music, shows 30 frames and reads `Audio.bgm_pos` thirty times without showing a frame, changing `Graphics.frame_count` between two of the readings
- **THEN** the thirty readings are equal

#### Scenario: Frames of a modal message
- **WHEN** a game of RGSS3 plays that file as music, shows 60 frames, shows a message box that the host dismisses after 30 frames, and reads `Audio.bgm_pos`
- **THEN** the position is 132304

#### Scenario: After a stop, measured on the original
- **WHEN** a game of RGSS3 plays that file as music, shows 24 frames, evaluates `Audio.bgm_stop` and reads `Audio.bgm_pos` at once and after 18 more frames
- **THEN** both readings are 33076

#### Scenario: During a fade, measured on the original
- **WHEN** a game of RGSS3 plays that file as music, evaluates `Audio.bgm_fade(1000)` and shows 60 frames
- **THEN** `Audio.bgm_pos` is 132304

#### Scenario: Formats without a position, measured on the original
- **WHEN** a game of RGSS3 plays as music a file that begins with the header of a Standard MIDI file, then a file of text, then an empty file, each with the `pos` 100000, and shows 60 frames after each
- **THEN** every call returns `nil`, raises nothing, and `Audio.bgm_pos` is 0 after each

#### Scenario: A file that is no sound after a music, measured on the original
- **WHEN** a game of RGSS3 plays that WAV file as music, shows 60 frames, plays a file of text as music and reads `Audio.bgm_pos`
- **THEN** the position is 0

### Requirement: A music effect holds the position of the music
In a game of RGSS3 `me_play` of a sound whose format is known SHALL hold the position of the
music, and not that of the background sound, for the length of the effect at its pitch,
counted in frames at the frame rate of the call. `me_stop` SHALL end the hold and `me_fade`
SHALL NOT. An effect whose format is not known SHALL hold nothing.

#### Scenario: Two seconds, measured on the original
- **WHEN** a game of RGSS3 at 60 frames per second plays the WAV file of 20 s at 44100 Hz in stereo as music, shows 60 frames, plays a WAV file of 2 s as a music effect and reads `Audio.bgm_pos` after 60, 120 and 180 more frames
- **THEN** the readings are 132304, 132304 and 297684

#### Scenario: The background sound goes on, measured on the original
- **WHEN** a game of RGSS3 plays that file as background sound, shows 60 frames, plays the effect of 2 s and shows 60 more frames
- **THEN** `Audio.bgs_pos` is 297684

#### Scenario: Stopped effect, measured on the original
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays a WAV file of 5 s as a music effect, shows 60 frames, evaluates `Audio.me_stop` and shows 60 more frames
- **THEN** `Audio.bgm_pos` is 297684

#### Scenario: Faded effect, measured on the original
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays the effect of 5 s, shows 60 frames, evaluates `Audio.me_fade(500)` and reads `Audio.bgm_pos` after 180 more frames and after 120 beyond those
- **THEN** the readings are 132304 and 297684

#### Scenario: Half the pitch
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays the effect of 2 s with the pitch 50 and reads `Audio.bgm_pos` after 240 and after 300 more frames
- **THEN** the readings are 132304 and 297684

#### Scenario: An effect whose format is not known
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays as a music effect a file that begins with the header of a Standard MIDI file and shows 60 more frames
- **THEN** `Audio.bgm_pos` is 297684

### Requirement: Probe of the format of a sound
`vkrgss-core` SHALL read from the bytes of a file, without an interpreter and without decoding
a sample, the sample rate, the channel count and the length in sample frames of a WAV file
of 8-bit or 16-bit PCM and of an Ogg Vorbis file, and of the latter the comments `LOOPSTART`
and `LOOPLENGTH`, whose names count in upper case only. For any other bytes it SHALL give
nothing, and no input SHALL make it panic.

#### Scenario: WAV
- **WHEN** the probe is given a generated WAV file of 16-bit PCM at 44100 Hz in stereo with 882000 sample frames, and one of 8-bit PCM at 22050 Hz in mono with 441000 bytes of data
- **THEN** it gives 44100, 2 and 882000 without tags, and 22050, 1 and 441000 without tags

#### Scenario: WAV that was cut short
- **WHEN** the probe is given a WAV file of 16-bit stereo whose data chunk declares 4000 bytes and of which 1000 bytes of data are there
- **THEN** it gives 250 sample frames

#### Scenario: Ogg Vorbis
- **WHEN** the probe is given generated Ogg pages that hold an identification header of 48000 Hz in stereo, a comment header without the two comments, and a last page with the granule position 960000
- **THEN** it gives 48000, 2 and 960000 without tags

#### Scenario: Tags, measured on the original
- **WHEN** the comment header holds `LOOPSTART=88200` and `LOOPLENGTH=176400`, then only the first, then only the second, then `loopstart=88200` and `looplength=176400`
- **THEN** the probe gives both tags, the start alone, the length alone, and no tag

#### Scenario: Comment header across pages
- **WHEN** the comment header is longer than one page holds and continues on the next page, with `LOOPSTART=5` as its last comment
- **THEN** the probe gives the tag 5

#### Scenario: Values that are no numbers
- **WHEN** the comment header holds `LOOPSTART=abc` and `LOOPLENGTH=`
- **THEN** the probe gives the format without tags

#### Scenario: Other bytes
- **WHEN** the probe is given the header of a Standard MIDI file, the first bytes of an MP3 file, the header of an ASF file, a file of text, no bytes at all, and a WAV file whose format tag is not 1
- **THEN** it gives nothing for each

#### Scenario: Any prefix
- **WHEN** the probe is given every prefix of the generated WAV file's first 64 bytes and of the generated Ogg file, and each of these files with one byte changed at every place of its headers
- **THEN** it returns for each, with a format or with nothing

### Requirement: Position of a streamed sound
The position model of `vkrgss-core` SHALL follow the rule measured on RGSS3 for a sound of
more than 529200 bytes of 16-bit PCM: a decoder position that starts at `pos`, advances by
15 chunks at the start and by one chunk per 187.5 ms at pitch 100, faster or slower by the
pitch, and wraps from the loop end to the loop start; the position is the decoder's less 16
chunks, not below 0. Time SHALL be counted in frames at their frame rate, in integers.

#### Scenario: Chunk, measured on the original
- **WHEN** the chunk is computed for 44100 Hz in stereo, 44100 Hz in mono, 22050 Hz in stereo, 22050 Hz in mono and 48000 Hz in stereo
- **THEN** it is 33076, 16538, 16540, 8268 and 36000 bytes

#### Scenario: Start, measured on the original
- **WHEN** a sound of 20 s is started at the `pos` 44100 with each of those five formats, and the one of 44100 Hz in stereo at 100001 and at 2147483647
- **THEN** the positions at once are 11024, 27562, 27560, 35832 and 8100, and 66925 and 2147450571

#### Scenario: Steps at 60 frames per second, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo is started at 0 and stepped by frames of 60 per second, read after 0, 12, 18, 24, 36, 48, 60, 72 and 84 frames
- **THEN** the positions are 0, 0, 0, 33076, 66152, 99228, 132304, 165380 and 198456

#### Scenario: Another frame rate
- **WHEN** the same sound is stepped by frames of 40 per second and read after 8, 15 and 40 frames
- **THEN** the positions are 0, 33076 and 132304

#### Scenario: Pitch, measured on the original
- **WHEN** the same sound is stepped 60 frames of 60 per second at the pitches 1, 50, 100, 150 and 200, and 180 frames at 49, 50 and 453
- **THEN** the positions are 0, 33076, 132304, 231532 and 297684, and 198456, 231532 and 2348396

#### Scenario: End of the file, measured on the original
- **WHEN** a sound of 4 s at 44100 Hz in stereo without tags is started at 0 and read after 75 and after 90 frames of 60 per second
- **THEN** the positions are 165380 and 0

#### Scenario: Both tags, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo with `LOOPSTART` 88200 and `LOOPLENGTH` 176400 is started at 600000 and read at once and after 60 frames of 60 per second, and started at 0 and read after 191 and 192 frames
- **THEN** the positions are 0 and 26704, and 496140 and 0

#### Scenario: Only the start tag, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo with `LOOPSTART` 88200 alone is started at 0 and read after 300 frames of 60 per second
- **THEN** the position is 826900

#### Scenario: A music of the run-time package, measured on the original
- **WHEN** a sound of 3020850 sample frames at 44100 Hz in stereo with `LOOPSTART` 280083 and `LOOPLENGTH` 2739337 is started at 11900000
- **THEN** the position at once is 909576

#### Scenario: Start behind the end, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo is started at 4000000 and read at once and after 24 frames of 60 per second
- **THEN** the positions are 3966924 and 4033076

#### Scenario: Just above the limit, measured on the original
- **WHEN** a sound of 132741 sample frames at 44100 Hz in stereo, 530964 bytes, is started at 100001
- **THEN** the position at once is 0

### Requirement: Position of a short sound
For a sound of at most 529200 bytes of 16-bit PCM the position model SHALL give `pos`
rounded down to a sample frame, plus the sample frames that the frames shown amount to at
the sound's rate and pitch, wrapped at the length of the sound, in bytes of 16-bit PCM. A
sound without a sample frame SHALL give 0.

#### Scenario: Start, measured on the original
- **WHEN** a sound of 3 s at 44100 Hz in stereo, 529200 bytes, is started at 100001
- **THEN** the position at once is 100000

#### Scenario: Frames
- **WHEN** that sound is started at 100000 and read after 60 frames of 60 per second, and started at 0 and read after 90 and after 180 frames
- **THEN** the positions are 276400, 264600 and 0

#### Scenario: Pitch and frame rate
- **WHEN** that sound is started at 0 with the pitch 50 and read after 40 frames of 40 per second
- **THEN** the position is 88200

#### Scenario: No sample frame
- **WHEN** a sound whose length is 0 is started at 100 and stepped 60 frames
- **THEN** the position is 0

### Requirement: Arguments of Input of RGSS3
In a game of RGSS3 `press?`, `trigger?` and `repeat?` SHALL take exactly one argument and
SHALL NOT raise for its class: a Symbol of a button's name, exactly as the constant is
named, or the Integer of that button in RGSS2 SHALL name the button, and any other value
SHALL give `false`. No method of the argument SHALL be called.

#### Scenario: Names and numbers, measured on the original
- **WHEN** in a game of RGSS3 each of the twenty buttons is held alone, a frame is shown and `Input.update` runs, and `press?`, `trigger?` and `repeat?` are asked for the button's constant, for its Symbol and for every Integer from -300 to 300
- **THEN** the constant and the Symbol give `true` for the three, and of the Integers exactly the button's number of RGSS2 gives `true`, for each of the three

#### Scenario: Anything else, measured on the original
- **WHEN** a game of RGSS3 with every button held, a frame shown and `Input.update` run evaluates `press?`, `trigger?` and `repeat?` of `:down`, `:Down`, `:NOPE`, `:""`, `"DOWN"`, `2.0`, `2.9`, `258`, `2**31 - 1`, `2**31`, `2**40`, `nil`, `true`, `false`, `[2]`, `Object.new` and `Input`
- **THEN** every answer is `false` and nothing is raised

#### Scenario: Count, measured on the original
- **WHEN** a game of RGSS3 evaluates `Input.press?`, `Input.trigger?(:A, :B)` and `Input.repeat?`
- **THEN** each raises `ArgumentError`

#### Scenario: The call of the standard scripts
- **WHEN** a game of RGSS3 evaluates `Input.update; Input.repeat?(:DOWN)` with nothing held
- **THEN** the result is `false` and nothing is raised

### Requirement: When Input of RGSS3 sees a key
In a game of RGSS3 the input state SHALL follow the rules of RGSS3 of `core-input`, and the
guest SHALL mark as shown every frame it shows: `Graphics.update` and each frame of `wait`,
`fadeout`, `fadein` and `transition`, and nothing else. An `Input.update` SHALL see what had
arrived when a frame was last shown.

#### Scenario: An update alone sees nothing new, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held before it does anything, calls `Input.update` twelve times, then `Graphics.update` and `Input.update`
- **THEN** `Input.press?(:C)` is `false` after each of the twelve and `true` after the last, and `Input.trigger?(:C)` is `true` after the last

#### Scenario: Seen after the next frame, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with DOWN held while its first `Graphics.update` waits for the tick, and then calls `Input.update`
- **THEN** `Input.press?(:DOWN)` and `Input.trigger?(:DOWN)` are `true`

#### Scenario: Calls that show no frame, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held before it does anything and evaluates `Graphics.frame_reset; Graphics.brightness = 255; Graphics.snap_to_bitmap.dispose; Graphics.freeze; Graphics.transition(0); Graphics.wait(0); Graphics.fadeout(0); Graphics.resize_screen(544, 416); Input.update; Input.press?(:C)`
- **THEN** the result is `false`

#### Scenario: Frames of waits and transitions, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held while the third frame of `Graphics.wait(3)` waits for its tick and then calls `Input.update`; and another game gets it while the second frame of `Graphics.freeze; Graphics.transition(2)` waits and then calls `Input.update`
- **THEN** after each `Input.update` `Input.press?(:C)` and `Input.trigger?(:C)` are `true`

### Requirement: Repeat and directions of Input of RGSS3
In a game of RGSS3 `trigger?`, `repeat?`, `dir4` and `dir8` SHALL follow "Input of RGSS3"
of `core-input` as measured: one button repeats at a time, at the 1st, 24th, 30th and
every 6th update after, and `dir4` resolves two axes by the axis held alone last.

#### Scenario: Repeat, measured on the original
- **WHEN** a game of RGSS3 holds DOWN from its start and runs 36 times `Graphics.update; Input.update`, collecting the numbers of the updates at which `Input.repeat?(:DOWN)` is `true`
- **THEN** they are `[1, 24, 30, 36]`

#### Scenario: The newest button repeats, measured on the original
- **WHEN** a game of RGSS3 holds DOWN from its start, holds DOWN and RIGHT from the frame of the 11th update on, and runs 40 updates, each after a `Graphics.update`
- **THEN** `Input.repeat?(:RIGHT)` is `true` at the 11th update, and `Input.repeat?(:DOWN)` is `false` at every update from the 11th to the 40th

#### Scenario: A and SHIFT, measured on the original
- **WHEN** a game of RGSS3 holds A and SHIFT from its start and runs 36 updates, each after a `Graphics.update`
- **THEN** `Input.repeat?(:SHIFT)` is `true` at the 1st, 24th, 30th and 36th, and `Input.repeat?(:A)` at none

#### Scenario: Directions, measured on the original
- **WHEN** a game of RGSS3 holds LEFT and UP as its very first input and asks `[Input.dir4, Input.dir8]` after a frame and an update; and, in another game, holds DOWN, then DOWN and RIGHT at a later frame; and holds LEFT and RIGHT
- **THEN** the answers are `[4, 7]`, `[6, 3]` after RIGHT was added, and `[0, 0]`

### Requirement: Method surface of Input in RGSS3
In a game of RGSS3 the public module functions of `Input` SHALL be exactly those of the
committed method list of RGSS3, with their arities, and its constants exactly the twenty of
the list, and the test of the list SHALL name `Input` as bound.

#### Scenario: Functions and constants, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Input.singleton_methods(false).sort, Input.constants.sort]`
- **THEN** the result is `[[:dir4, :dir8, :press?, :repeat?, :trigger?, :update], [:A, :ALT, :B, :C, :CTRL, :DOWN, :F5, :F6, :F7, :F8, :F9, :L, :LEFT, :R, :RIGHT, :SHIFT, :UP, :X, :Y, :Z]]`

#### Scenario: Bound
- **WHEN** the test of the list of RGSS3 runs after this change
- **THEN** it checks `Input` beside the classes and modules bound before, and passes

### Requirement: Input of RGSS1 and RGSS2 is unchanged by RGSS3
`Input` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: marking shown
frames SHALL change nothing there, and every test of the two binding packages that passed
before SHALL pass under its name with its expected values unedited, over both interpreters.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding` and of `vkrgss-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS2 gets a snapshot with C held before it does anything and evaluates `Input.update; [Input::DOWN, Input::F9, Input.press?(Input::C)]`
- **THEN** the result is `[2, 29, true]`, over both interpreters
