## ADDED Requirements

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
