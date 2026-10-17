## ADDED Requirements

### Requirement: The guest runs in a process on Android
On Android the app SHALL run the guest of every game in a process of its own, started from
the guest programs the APK installs, confined by the seccomp filter, so that games of RGSS1,
RGSS2 and RGSS3 run and save states work.

#### Scenario: A game of VX Ace on a phone
- **WHEN** `cargo xtask android-run` runs Crysalis on each phone with frames to dump
- **THEN** the frames are identical to those of the same run on this machine

#### Scenario: Save and load a state
- **WHEN** a game on a phone saves a state, goes on, and loads the state
- **THEN** the frame after the load equals the frame after the save, as on the desktop

#### Scenario: The app ends
- **WHEN** the app is stopped with `am force-stop` while a game and a parked state run
- **THEN** no guest process of the app is left
