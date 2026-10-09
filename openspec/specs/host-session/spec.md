# host-session Specification

## Purpose
Defines what the host does with the guest's messages and the user's input: frame pacing, input
delivery, reset and quit, freeze and transition, and the windowed and headless ways of running a
session.

## Requirements

### Requirement: Frame pacing
The host SHALL send the tick for a frame one frame period after the previous tick, or at once if
that moment has already passed, where the period is one second divided by the frame rate of that
frame's packet. Lateness SHALL NOT be made up: after a late frame the following ticks are again
one period apart. Time SHALL be read from a clock that tests can replace.

#### Scenario: Steady rate
- **WHEN** a guest sends each frame as soon as it has the previous tick, at frame rate 40
- **THEN** consecutive ticks are 25 milliseconds apart on the session's clock

#### Scenario: Late frame
- **WHEN** a frame arrives 100 milliseconds after the previous tick at frame rate 40
- **THEN** its tick is sent at once and the next tick is due 25 milliseconds after it

#### Scenario: Frame rate change
- **WHEN** a packet carries frame rate 20 after packets at 40
- **THEN** the tick for that packet is due 50 milliseconds after the previous tick

### Requirement: Fast-forward and frame reset
While fast-forward is engaged the host SHALL send each tick as soon as its frame has been
rendered. After a frame-reset message the tick of the next frame SHALL be due one period after
that frame arrives, regardless of earlier ticks.

#### Scenario: Fast-forward
- **WHEN** fast-forward is engaged and frames arrive back to back
- **THEN** each tick is sent without waiting for the frame period

#### Scenario: Frame reset
- **WHEN** a frame-reset message is followed by a frame that arrives late
- **THEN** that frame's tick is due one period after its arrival, not at once

### Requirement: Input delivery
The host SHALL translate key presses and releases into the RGSS1 buttons through a key map and
SHALL send the guest a complete input snapshot whenever the set of held buttons changes, and
only then. Losing keyboard focus SHALL release all buttons.

#### Scenario: Press and release
- **WHEN** the Down arrow is pressed and later released
- **THEN** the guest receives a snapshot with only DOWN held and later a snapshot with nothing held

#### Scenario: Two keys, one button
- **WHEN** Space and Enter are both held and Space is released
- **THEN** no new snapshot is sent, because C is still held

#### Scenario: Focus lost
- **WHEN** keys are held and the window loses focus
- **THEN** the guest receives a snapshot with nothing held

### Requirement: Default key map
By default the arrow keys and numeric-pad 2, 4, 6 and 8 SHALL be the directions; Space, Enter
and C SHALL be C; Escape, numeric-pad 0, Insert and X SHALL be B; Shift and Z SHALL be A; the
keys A, S and D SHALL be X, Y and Z; Q and Page Up SHALL be L; W and Page Down SHALL be R;
Shift, Control and Alt SHALL be SHIFT, CTRL and ALT; and F5 to F9 SHALL be themselves.

#### Scenario: One key, two buttons
- **WHEN** Shift is pressed
- **THEN** the snapshot has both A and SHIFT held

#### Scenario: Letter keys
- **WHEN** the key X is pressed
- **THEN** the snapshot has B held, not the button named X

#### Scenario: Function key
- **WHEN** F9 is pressed
- **THEN** the snapshot has F9 held

### Requirement: Reset, quit and shutdown
Pressing F12 SHALL send the guest one reset request per press. A request to close the window
SHALL send the guest a quit request. When the guest sends its shutdown message, or its endpoint
disconnects, the session SHALL end.

#### Scenario: Reset key
- **WHEN** F12 is pressed, held and released
- **THEN** the guest receives exactly one reset request

#### Scenario: Closing the window
- **WHEN** the user closes the window
- **THEN** the guest receives a quit request and the session keeps running until the guest shuts down

#### Scenario: Guest shuts down
- **WHEN** the guest sends its shutdown message
- **THEN** the session ends with a success status

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

### Requirement: Title and modal messages
A title message SHALL set the window title. A modal message SHALL be written to the error
stream and answered with a dismissal at once; drawing a message panel is the guest's business.

#### Scenario: Title
- **WHEN** the guest sends a title
- **THEN** the window title becomes that text

#### Scenario: Modal
- **WHEN** the guest sends a modal message
- **THEN** the text appears on the error stream and the guest receives a dismissal

### Requirement: Rejected frames
When the compositor rejects a frame packet, the host SHALL report the error with its details on
the error stream, keep the previous image on screen and still send the tick. In a headless run
the session SHALL end with a failure status after reporting it.

#### Scenario: Windowed session continues
- **WHEN** a packet with an invalid op arrives in a windowed session
- **THEN** the error is reported, the display is unchanged and the guest still gets its tick

#### Scenario: Headless session fails
- **WHEN** a packet with an invalid op arrives in a headless run
- **THEN** the run ends with a failure status and the error on the error stream

### Requirement: Headless run
The host SHALL be able to run a session without a window or display: on a virtual clock that
never waits, with input taken from a script that names the buttons held from given frame numbers
on, for a given number of frames or until the guest shuts down, writing chosen frames as PNG
files at the logical size. Two headless runs of the same guest with the same script SHALL write
identical files.

#### Scenario: Frame dump equals the reference
- **WHEN** the demo guest runs headless and one of its frames is dumped
- **THEN** the PNG's pixels equal what the CPU reference compositor produces for the same messages

#### Scenario: Scripted input
- **WHEN** the script holds RIGHT from frame 10 to frame 20
- **THEN** the guest receives a snapshot with RIGHT held before the tick of frame 10 and one with nothing held before the tick of frame 20

#### Scenario: Repeatable
- **WHEN** the same headless run is made twice
- **THEN** the dumped files are byte-identical

#### Scenario: Frame limit
- **WHEN** a headless run is limited to 30 frames and the guest never shuts down
- **THEN** the run sends a quit request after the 30th frame and ends

### Requirement: Audio commands
The session SHALL pass every audio command of the guest to its audio sink in the order
received and without waiting for it. A headless run SHALL use a sink that plays nothing and
reports nothing.

#### Scenario: Order
- **WHEN** the guest sends play, fade and stop commands between frames
- **THEN** a recording sink has received the same commands in the same order

#### Scenario: Headless
- **WHEN** a game that plays music runs headless
- **THEN** no audio device is opened and no audio report is printed

### Requirement: Input recording and replay
The host SHALL be able to record the input snapshot it delivers for every frame into a file
together with the run's seed, and to replay such a file: during a replay the recorded snapshot
of each frame SHALL be delivered in place of live input, and after the last recorded frame live
input SHALL apply again in a windowed run and no button SHALL be held in a headless run. A
file that is not a recording SHALL be refused with an error naming it.

#### Scenario: Round trip
- **WHEN** a headless run with an input script is recorded and the recording is replayed in a second headless run with the same seed
- **THEN** the frame dumps of both runs are byte-identical

#### Scenario: Not a recording
- **WHEN** a replay is asked for with a file of other content
- **THEN** the run fails before the first frame and names the file

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

### Requirement: Configurable key map
The key map SHALL be replaceable per button: a configuration that lists keys for a button
SHALL make exactly those keys hold it, and buttons it does not mention SHALL keep their default
keys. An unknown button or key name SHALL be an error that names it.

#### Scenario: Remapped button
- **WHEN** the configuration maps the button C to the key J only
- **THEN** J holds C, Space no longer holds C, and the arrow keys still hold the directions

#### Scenario: Unknown key name
- **WHEN** the configuration names a key that does not exist
- **THEN** loading it fails with an error naming that key

### Requirement: Gamepads
A connected gamepad SHALL hold buttons in the input snapshot together with the keyboard: its
directional pad and its left stick beyond a dead zone SHALL be the directions, and its buttons
SHALL map to RGSS buttons by a default table that the configuration can replace per button. A
pad connected or removed during a run SHALL be picked up without a restart, and a removed pad
SHALL release everything it held. A machine without gamepad support SHALL run normally.

#### Scenario: Directional pad
- **WHEN** a pad reports its directional pad held to the right
- **THEN** the snapshot has RIGHT held

#### Scenario: Stick dead zone
- **WHEN** the left stick is at 0.1 and then at 0.9 on the horizontal axis
- **THEN** RIGHT is not held and then held

#### Scenario: Keyboard and pad together
- **WHEN** the keyboard holds LEFT and a pad holds its button mapped to C
- **THEN** the snapshot has both held

#### Scenario: Removal
- **WHEN** a pad that holds a button is removed
- **THEN** the next snapshot no longer has that button held
