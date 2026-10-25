# supported-platforms Specification

## Purpose
Defines which platforms vkrgss supports and plans, in what order, and how each is accepted.

## Requirements

### Requirement: Planned platforms and their order
`docs/decisions.md` SHALL name Linux x86_64 as the platform supported now and SHALL list the
planned platforms in the order ARM Linux, macOS, Android, Windows, each as a milestone row
P1 to P4 whose status follows recorded evidence.

#### Scenario: Scope
- **WHEN** the section "Goal and scope" of `docs/decisions.md` is read
- **THEN** it names Linux x86_64 as supported now and the four planned platforms in that order

#### Scenario: Milestone rows
- **WHEN** the section "Milestones" is read
- **THEN** it has rows P1 to P4, each "Not started", with its outcome and acceptance, below the rows A0 to A3, which are unchanged

### Requirement: Acceptance of a platform
`docs/decisions.md` SHALL state for each planned platform how it is accepted; for ARM Linux,
that frames and saves on aarch64 are identical byte for byte to those of x86_64 for the same
games, frames and input.

#### Scenario: ARM Linux
- **WHEN** the row P1 is read
- **THEN** its acceptance compares aarch64 with x86_64 byte for byte, frames and saves

#### Scenario: Windows without save states
- **WHEN** the decisions on Windows are read
- **THEN** they say that save states are not supported there and why

### Requirement: Recorded decisions of the grilling of 2026-10-15
`docs/decisions.md` SHALL record the decisions of `design.md` of this change on floats in
saves, Vulkan, display, audio, Linux packages, CI, test devices and the order of work.

#### Scenario: Each decision is there
- **WHEN** `docs/decisions.md` is read against the sections of `design.md` of this change
- **THEN** each decision is found, in the section where it belongs

### Requirement: The interpreters build and are tested on Android
Both interpreter crates SHALL build for `aarch64-linux-android` at API level 34 or later,
and their tests SHALL pass on a phone over adb, except those that need the host, each of
which names the reason. A build below API level 34 SHALL fail with a message that names
the minimum.

#### Scenario: Upstream suites on a phone
- **WHEN** the upstream suites of both interpreters run on the Pixel 7a and on the Galaxy S24+ through the adb runner
- **THEN** they pass, outside the cases listed for Android with the reason

#### Scenario: IO at the end of a file
- **WHEN** Ruby 1.8.7 on a phone reads an empty file and a closed pipe with `eof?` and `readpartial`
- **THEN** `eof?` is true and `readpartial` raises `EOFError` without blocking

#### Scenario: crypt
- **WHEN** `"x".crypt("ab")` is evaluated on Android
- **THEN** it raises `NotImplementedError`

#### Scenario: Too old an API level
- **WHEN** an interpreter crate is built for Android with API level 31
- **THEN** the build fails and names API level 34

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

### Requirement: The workspace builds and is tested on macOS
The workspace SHALL build for `aarch64-apple-darwin`, and its tests SHALL pass there on
MoltenVK, except those kept to Linux, each of which names the reason.

#### Scenario: Vulkan backend on the Apple GPU
- **WHEN** the tests of `vkrgss-vk` run on a Mac with MoltenVK and the Apple GPU as the test device
- **THEN** they pass, the differential tests against the CPU reference included

#### Scenario: Interpreters
- **WHEN** the tests of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` run on the Mac
- **THEN** they pass, and `RUBY_PLATFORM` is `aarch64-darwin`

#### Scenario: Errors of Math on macOS
- **WHEN** Ruby 1.8.7 evaluates `Math.log(0)` and `Math.atanh(1)` on the Mac
- **THEN** each raises `Errno::ERANGE`, as on Linux

#### Scenario: Linux unchanged
- **WHEN** the gate runs on Linux
- **THEN** it passes as before

### Requirement: A self-contained macOS app
`cargo xtask app` SHALL build `vkrgss.app`, which runs on a Mac without Homebrew's Vulkan
loader or MoltenVK installed and without `DYLD_*` variables, and SHALL ask for a game
directory when started without one.

#### Scenario: No Homebrew needed
- **WHEN** `otool -L` is run on every executable and library of the bundle
- **THEN** none names a path under `/opt/homebrew`

#### Scenario: Headless frame from the bundle
- **WHEN** the bundle's `vkrgss` runs a generated game headless with a frame to dump, from a shell without `DYLD_*` variables
- **THEN** the dumped frame is identical to that of the development build

#### Scenario: No game directory
- **WHEN** the app starts without a game directory and the folder chooser is cancelled
- **THEN** the app ends without an error dialog and with status 0

### Requirement: The comparison games give the same output on every platform
With the same game, frames, input, RTP, fonts and built-in scripts, the frames and saves of the
comparison games SHALL be identical on x86_64 Linux, macOS and the two Android phones;
`cargo xtask xarch` SHALL run and compare them.

#### Scenario: All platforms
- **WHEN** `cargo xtask xarch` runs the ten comparison games on this machine, on the Mac and on both phones, and compares each with this machine
- **THEN** every frame and every save is identical

#### Scenario: Arial on Android
- **WHEN** a script on a phone evaluates `Font.exist?("Arial")`
- **THEN** it is true

### Requirement: A release APK
`cargo xtask apk --release` SHALL build an APK signed with the keystore that the environment
names, with the workspace's version, and SHALL refuse without it; the release workflow SHALL
attach that APK to the release of a version tag when the signing secrets exist.

#### Scenario: Signed with the given key
- **WHEN** `cargo xtask apk --release` runs with a throwaway keystore in the environment
- **THEN** `apksigner verify --print-certs` names that key and the manifest's `versionName` is the workspace version

#### Scenario: No keystore
- **WHEN** it runs without the variables
- **THEN** it fails and names them

### Requirement: Player-side pieces on Android
The app SHALL import the built-in scripts through the picker, pause sound while in the
background, and run without validation errors.

#### Scenario: Built-in scripts imported
- **WHEN** an `rpg_builtin.rb` is imported through the library screen and a game of RGSS1 starts
- **THEN** the game runs with it, without a file in the external files directory

#### Scenario: Sound in the background
- **WHEN** the app goes to the background while music plays
- **THEN** no sound is output until the app returns

#### Scenario: Validation on the phones
- **WHEN** `cargo xtask android-run --validation` runs the comparison games on both phones
- **THEN** no validation error is logged

### Requirement: The macOS app needs no newer macOS than it declares
Every executable and library in `vkrgss.app` SHALL be built for the minimum macOS that its
`Info.plist` declares, macOS 14, and `cargo xtask app` SHALL fail when one is not.

#### Scenario: Minimum of every file
- **WHEN** `cargo xtask app` has built the bundle
- **THEN** the minimum macOS of every Mach-O file in it is at most 14.0

#### Scenario: A file built for a newer macOS
- **WHEN** the check is given a library whose minimum is 27.0
- **THEN** it fails and names the file and its minimum

### Requirement: Name and icon of the Android app
The Android app SHALL be labelled `RGSS Player` and SHALL carry the project's launcher icon as
an adaptive icon with a monochrome layer and as legacy and round icons per density; its
package name SHALL stay `io.github.xuyanwen2012.vkrgss`.

#### Scenario: Name and icon
- **WHEN** `aapt2 dump badging` reads an APK that `cargo xtask apk` built
- **THEN** the application label is `RGSS Player`, the package is `io.github.xuyanwen2012.vkrgss`, and the application icon is a resource of the APK
