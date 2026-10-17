## ADDED Requirements

### Requirement: A game runs on Android
An APK built by `cargo xtask apk` SHALL start on Android 14 or later, open a window, run a
game of RGSS1 or RGSS2 from its files directory with the guest on a thread, and present
its frames through the Vulkan surface of that window.

#### Scenario: Frames on a phone
- **WHEN** `cargo xtask android-run` runs a game on the Pixel 7a and on the Galaxy S24+ with frames to dump
- **THEN** the dumped frames are identical to those of the same game, frames and input on this machine with `--guest thread --headless`

#### Scenario: Pause and resume
- **WHEN** the activity is sent to the background and brought back while the game runs
- **THEN** the game goes on and frames are presented again, without a validation error or a crash

#### Scenario: Game of RGSS3
- **WHEN** the files directory holds a game of RGSS3
- **THEN** the app says that this runtime is not supported on Android yet and ends
