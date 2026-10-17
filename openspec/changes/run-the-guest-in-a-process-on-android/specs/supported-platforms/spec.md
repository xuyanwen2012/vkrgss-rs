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

## MODIFIED Requirements

### Requirement: A game runs on Android
An APK built by `cargo xtask apk` SHALL start on Android 14 or later, open a window, run a
game of RGSS1, RGSS2 or RGSS3 from its files directory with the guest in a process of its
own, and present its frames through the Vulkan surface of that window.

#### Scenario: Frames on a phone
- **WHEN** `cargo xtask android-run` runs a game on the Pixel 7a and on the Galaxy S24+ with frames to dump
- **THEN** the dumped frames are identical to those of the same game, frames and input on this machine with `--headless`

#### Scenario: Pause and resume
- **WHEN** the activity is sent to the background and brought back while the game runs
- **THEN** the game goes on and frames are presented again, without a validation error or a crash

#### Scenario: Game of RGSS3
- **WHEN** the files directory holds a game of RGSS3
- **THEN** the app runs it with the guest executable of RGSS3 in a process, as a game of RGSS1 or RGSS2 runs with its own
