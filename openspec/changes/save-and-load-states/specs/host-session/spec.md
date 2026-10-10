# Spec Delta

## ADDED Requirements

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
