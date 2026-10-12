## ADDED Requirements

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
