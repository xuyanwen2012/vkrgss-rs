## MODIFIED Requirements

### Requirement: Reset, quit and shutdown
Pressing F12 SHALL send the guest one reset request per press. A request to close the window
SHALL send the guest a quit request. When the guest sends its shutdown message, or its endpoint
disconnects, the session SHALL end. A guest that has not shut down 5 seconds after the first
quit request SHALL be given up on: the session SHALL end with a failure that says so.

#### Scenario: Reset key
- **WHEN** F12 is pressed, held and released
- **THEN** the guest receives exactly one reset request

#### Scenario: Closing the window
- **WHEN** the user closes the window
- **THEN** the guest receives a quit request and the session keeps running until the guest shuts down or 5 seconds have passed

#### Scenario: Guest shuts down
- **WHEN** the guest sends its shutdown message
- **THEN** the session ends with a success status

#### Scenario: Guest does not shut down
- **WHEN** the window is closed and the guest sends nothing for 5 seconds of the session's clock
- **THEN** the session ends with the failure that the guest was given up on

#### Scenario: Script in an endless loop
- **WHEN** the window of a windowed run is closed while its guest never reads its link, once with a guest thread that loops without reading and once with a guest process whose script runs `loop {}`
- **THEN** the run ends within 10 seconds with the failure that the guest was given up on, and no guest process is left

### Requirement: Injected keys act as physical keys
A key the guest injects SHALL be held in the raw keys of the snapshots the session sends
from then on, SHALL press the buttons the key map gives the physical key of that code, and
SHALL cause the action that key has in the original runtime, but none of the program's own
shortcuts, until the guest releases it, a state is loaded or the game is reset. A session
that replays a recording SHALL ignore injected keys.

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

#### Scenario: Program shortcuts
- **WHEN** the guest injects presses of F3 (0x72), F4 (0x73), Tab (0x09) and F11 (0x7A)
- **THEN** the session sends no state request, does not engage fast-forward, reports no full screen action, and the snapshots hold those raw keys

#### Scenario: Reset by an injected key
- **WHEN** the guest injects a press of F12 (0x7B)
- **THEN** the guest receives one reset request

## ADDED Requirements

### Requirement: Notices do not move the clock of a headless run
A headless run SHALL move its clock only to the times of ticks; the end of a notice SHALL
not move it.

#### Scenario: Save and load
- **WHEN** a headless run of a fixed number of frames is made once without and once with a schedule that saves and loads states
- **THEN** both report the same elapsed time of the clock for the same number of frames
