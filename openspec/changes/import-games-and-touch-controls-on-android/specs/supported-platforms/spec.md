## ADDED Requirements

### Requirement: Games are imported and played by touch on Android
The Android app SHALL import a game or the RTP from a zip chosen in the system picker into
its private storage, list the imported games and start the chosen one, and SHALL offer
on-screen buttons that press the runtime's keys, hidden while a gamepad is connected.

#### Scenario: Import a zip
- **WHEN** a zip of a game whose top level is one directory with `Game.ini` is imported
- **THEN** the game appears in the list under the title of its `Game.ini` and starts when chosen

#### Scenario: A path that leaves the directory
- **WHEN** a zip holds an entry named `../escape.txt`
- **THEN** the import is refused and nothing is written outside the game's directory

#### Scenario: Buttons move the player
- **WHEN** the down button is held on the screen for the frames of a key schedule
- **THEN** the dumped frame equals that of the desktop run with the same schedule of the down key

#### Scenario: Gamepad hides the buttons
- **WHEN** a gamepad key event arrives
- **THEN** the on-screen buttons are not drawn until the screen is touched again
