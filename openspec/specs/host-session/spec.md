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
rendering and answering later frames. After a transition message of duration N the
k-th following frame SHALL be shown at step k-1 of N equal steps from the held frame to the
newest, and frames SHALL be shown normally from the frame after the N-th. The transition SHALL
be a cross-fade, or with a mask bitmap the presenter's masked mix. When nothing is frozen, the
picture on screen SHALL stay for those frames.

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
stream. A headless run SHALL answer it with a dismissal at once; a windowed run SHALL answer
when a key mapped to C or B is newly pressed or a mouse button is clicked, and SHALL NOT show
that press to the game as input. Frames SHALL keep being shown and answered while a modal
message is pending.

#### Scenario: Title
- **WHEN** the guest sends a title
- **THEN** the window title becomes that text

#### Scenario: Modal
- **WHEN** the guest sends a modal message in a headless run
- **THEN** the text appears on the error stream and the guest receives a dismissal

#### Scenario: Waiting for the player
- **WHEN** the guest sends a modal message in a windowed session, three frames follow, and then the key for C is pressed
- **THEN** no dismissal is sent before the press, one is sent after it, the three frames are answered with ticks, and no snapshot shows C held by that press

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

### Requirement: Initial window size and fullscreen
A windowed run SHALL open its window with a client area of the frame size times the largest
whole number at which it fits the monitor's work area, at least 1. Alt+Enter and F11 SHALL
toggle borderless fullscreen on the monitor the window is on, and leaving fullscreen SHALL
restore the earlier window size. Neither key SHALL reach the game as a button while it toggles.

#### Scenario: Size computation
- **WHEN** the frame is 640 by 480 and the work area is 1920 by 1040
- **THEN** the client area is 1280 by 960, and for a work area of 600 by 400 it is 640 by 480

#### Scenario: Toggle
- **WHEN** Alt+Enter is pressed twice
- **THEN** the window is fullscreen after the first and back at its earlier size after the second, and the snapshots of those frames have neither C nor ALT newly held by that key press

### Requirement: Transition step values
For the k-th frame of a transition of duration N, shown at step `s = k - 1`, the session SHALL
give the presenter the cross-fade weight `an = 255 * s / N` and, for a masked transition, the
threshold `T = 256 * s / N - 1` in integer division, floored at 0. At step 0 the display SHALL
show the held frame alone, with or without a mask.

#### Scenario: Last step
- **WHEN** a transition of duration 8 shows its eighth frame
- **THEN** the weight is 223, and for durations 3 and 5 the thresholds of the last step are 169 and 203

#### Scenario: Intermediate thresholds
- **WHEN** a masked transition of duration 20 shows its frames
- **THEN** the thresholds of steps 1, 2, 6 and 11 are 11, 24, 75 and 139

#### Scenario: First step of a masked transition
- **WHEN** a masked transition of duration 5 with vagueness 40 shows its first frame
- **THEN** the display shows the held frame at every pixel, whatever the mask

### Requirement: Mask bitmaps reach the display
A transition picture that the guest loaded SHALL be available to the display when the first
frame of the transition is shown, so that a mask that exists is never replaced by a
cross-fade.

#### Scenario: Mask file of a game
- **WHEN** a generated game with a picture in `Graphics/Transitions` runs `Graphics.freeze`, draws, and calls `Graphics.transition(10, "Graphics/Transitions/<name>")` headless
- **THEN** no report about a missing mask is printed and the dumped frame inside the transition shows the mask's pattern

### Requirement: Raw keys and pointer from the window
A windowed session SHALL set in each snapshot the virtual-key bits of the keys and mouse
buttons that are held, by the Windows virtual-key code of each physical key, and the pointer
position mapped from the window into the frame, or none when the pointer is outside the
picture. Headless runs SHALL deliver neither unless a replayed recording holds them.

#### Scenario: Letter and modifier
- **WHEN** the keys A and left Shift are held
- **THEN** the snapshot has the virtual keys 0x41, 0x10 and 0xA0 set

#### Scenario: Pointer in a letterboxed window
- **WHEN** a 640 by 480 frame is shown at scale 2 with bars of 100 pixels left and right and the pointer is at window position (300, 200)
- **THEN** the snapshot's pointer is (100, 100), and at window position (50, 200) there is none

### Requirement: Raw keys in recordings
Recordings SHALL store the raw keys and the pointer of every frame, and recordings made before
this change SHALL still replay.

#### Scenario: Old recording
- **WHEN** a recording of format version 1 is replayed
- **THEN** it replays as before with no raw keys

### Requirement: Frame time statistics
On request the host SHALL report a summary of where frames spend their time: one row per
stage the backend measures and one for the time spent waiting for the guest, each with count,
mean, smallest and largest. A windowed run SHALL report periodically, a headless run once at
its end. The measurement SHALL NOT change pacing or anything the guest observes.

#### Scenario: Headless summary
- **WHEN** a game runs headless for 50 frames with the statistics requested
- **THEN** the error stream has one summary with a row for the guest and a row for each processor stage, each counting the frames measured

#### Scenario: Not requested
- **WHEN** the same run is made without the request
- **THEN** no summary is written

#### Scenario: Same frames
- **WHEN** the same headless run dumps a frame with and without the request
- **THEN** the two dumps are equal

### Requirement: Reports carry a severity
A report of the session or of a headless run SHALL state whether it is an error, a warning, or
text an option asked for. A rejected frame and a display that cannot be frozen SHALL be
errors.

#### Scenario: Rejected frame at error level
- **WHEN** a headless run whose guest sends a frame the backend rejects is made with `VKXP_LOG=error`
- **THEN** the error stream names the rejected frame

#### Scenario: Degradation is a warning
- **WHEN** a transition names a mask bitmap that does not exist
- **THEN** the report is a warning and the transition is a plain cross-fade

#### Scenario: Text unchanged
- **WHEN** the existing tests that read report lines run
- **THEN** the lines they expect are the lines written

### Requirement: Guest time is split
When frame time statistics are requested, the guest SHALL report, per frame, the time it
spends in bitmap operations, in text drawing, in frame extraction and in the rest of its
frame, as rows with count, mean, smallest and largest. Without the request the guest SHALL
measure nothing, and the measurement SHALL NOT change anything a script observes or any frame.

#### Scenario: Rows of a headless run
- **WHEN** a game that draws text into a bitmap every frame runs headless for 50 frames with the statistics requested
- **THEN** the error stream has rows for the guest's scripts, bitmap operations, text and extraction, each counting the frames measured

#### Scenario: Not requested
- **WHEN** the same run is made without the request
- **THEN** no such row is written

#### Scenario: Same frames
- **WHEN** the same run dumps a frame with and without the request
- **THEN** the two dumps are equal

### Requirement: State slots and keys
The session SHALL have ten state slots, 0 to 9, with slot 0 chosen at the start. F3 SHALL
request a save into the chosen slot and F4 a load from it; with Shift they SHALL choose the
next and the previous slot. These presses SHALL hold no button of the game. The outcome of a
request and a change of slot SHALL be shown in the window title for about two seconds.

#### Scenario: Save and load keys
- **WHEN** F3 and later F4 are pressed
- **THEN** the session sends a save request and then a load request for slot 0, and no snapshot has a button newly held by those presses

#### Scenario: Choosing a slot
- **WHEN** Shift+F3 is pressed twice and then F3
- **THEN** the save request names slot 2

#### Scenario: Not available
- **WHEN** F3 is pressed while input is recorded or replayed, or while the guest runs as a thread
- **THEN** no request is sent and the title says why

#### Scenario: Modal message
- **WHEN** F3 is pressed while a modal message is shown
- **THEN** no request is sent

### Requirement: Loading a state resets the host's side
When the session learns that a state was loaded it SHALL make the display forget every bitmap
and table, end a freeze or transition, drop a modal message, stop the four audio channels,
restart the frame pacing and send the current input again.

#### Scenario: Stale resources
- **WHEN** a state is loaded and the next frame creates a bitmap with an id the display held before
- **THEN** the frame is rendered without a rejected frame

#### Scenario: Mid transition
- **WHEN** a state is loaded while a transition is running
- **THEN** the next frame is shown alone, at its own time

### Requirement: Scheduled state requests
A headless run SHALL take a schedule of state requests by frame, and SHALL make each request
at its frame as the keys would.

#### Scenario: Back to the saved frame
- **WHEN** a generated game that draws from a counter and from `rand` is run headless with the guest in a process, a save into slot 0 at frame 60 and a load at frame 100
- **THEN** the frame dumped at 100 + k equals the frame dumped at 60 + k of a run without the schedule, for k from 1 to 30

#### Scenario: Loading twice
- **WHEN** the schedule saves at frame 60 and loads at frames 100 and 140
- **THEN** the frames dumped at 110 and 150 are equal

#### Scenario: Empty slot
- **WHEN** the schedule loads slot 3 without a save
- **THEN** the run reports that the slot holds no state and continues to its end with status 0

### Requirement: State notices end and failures are told apart
A notice about states SHALL leave the window title when its time is over, also when the guest
sends no frame meanwhile. A failed answer SHALL be acted on by what it says failed: only a
failed load SHALL end the wait for a load of its slot.

#### Scenario: No frames
- **WHEN** a notice is shown, no frame arrives and the session is polled at the time it names as its next deadline
- **THEN** that deadline is the end of the notice and the title is the game's title again

#### Scenario: Failed save while a load waits
- **WHEN** a load of slot 3 waits for its answer and a failed save for slot 3 arrives
- **THEN** the title says that no state could be saved, no tick is sent, and the following loaded answer resets the host's side as usual

#### Scenario: Failed load
- **WHEN** a load of slot 3 waits for its answer and a failed load for slot 3 arrives
- **THEN** the title says that the slot holds no state and the ticks go on

### Requirement: Music goes on after a load
The session SHALL have the audio sink remember the slot when a state was saved into it,
and recall the slot when its state was loaded, after the channels were stopped.

#### Scenario: Save, then load
- **WHEN** the saved answer for slot 2 arrives, and later the loaded answer for slot 2
- **THEN** the sink was asked to remember slot 2 at the first, and at the second to stop the four channels and then to recall slot 2

### Requirement: Injected keys act as physical keys
A key the guest injects SHALL be held in the raw keys of the snapshots the session sends
from then on, SHALL press the buttons the key map gives the physical key of that code, and
SHALL cause the action that key causes, until the guest releases it, a state is loaded or
the game is reset. A session that replays a recording SHALL ignore injected keys.

#### Scenario: Raw key and button
- **WHEN** the guest injects a press of key 0x0D with the default key map and the next tick is sent
- **THEN** the snapshot ahead of that tick has key 0x0D held and the button C

#### Scenario: Release
- **WHEN** the guest then injects the release
- **THEN** the snapshot of the following tick has neither

#### Scenario: Full screen
- **WHEN** the guest injects a press of key 0x12 and then a press of key 0x0D
- **THEN** the session reports the action that switches full screen, once

#### Scenario: Replay
- **WHEN** a session with a replayer receives an injected press
- **THEN** the snapshots it sends are those of the recording

### Requirement: Pointer visibility
The session SHALL pass on to its display whether the pointer is to be seen when the guest
says so, and a window SHALL hide or show the pointer over its picture accordingly.

#### Scenario: Hidden
- **WHEN** the guest says the pointer is not to be seen
- **THEN** the display was told to hide it, once

### Requirement: Mask threshold is not negative
The threshold of a step of a masked transition SHALL NOT be below 0.

#### Scenario: Long transition
- **WHEN** step 1 of a masked transition of 257 frames is shown
- **THEN** its threshold is 0
