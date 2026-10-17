# Design

## Context

- `crates/vkrgss/src/process.rs` (`guest_executable_of`) finds the guest executable beside
  the program; the relay, the record of a guest process and the socket are there too.
- `cargo xtask apk` and `cargo xtask android-run` exist (change `run-a-game-on-android`);
  both guest programs link for Android (change `build-the-interpreters-for-android`).
- Phones on USB, free to use for the test APK: Pixel 7a `3A021JEHN02756`, Galaxy S24+
  `R5CY21Y3VEV`. Material of the measurement: `../../vkxp/target/p3-android/` (read only).

## Decisions

- **Where the guest lies.** On Android, the directory of the guest programs is the app's
  `nativeLibraryDir` (from `AndroidApp`/the activity's application info through JNI, or
  `/proc/self/maps` of the app's own `.so`, whichever is simpler and reliable); the file
  names are those of the `.so` entries. The guest program keeps its normal `main`: the file
  is an executable that merely has a `.so` name. Check that the installer extracts it with
  the execute bit and that `execve` from the app works under the app's SELinux domain; if it
  does not, record what fails and fall back to `fork` without `exec` of the app process,
  stated in `design.md`, as a follow-up.
- **The app's own process.** The app no longer links the interpreter if every game goes
  through a guest process; keep the thread path only if removing it would grow this change
  beyond the above.
- **Save states.** As on the desktop: the guest's copier forks it. The app's low-memory
  killer may end parked copies: when a parked copy is gone, loading it reports an error
  instead of hanging; test by killing a copy by PID.
- **Acceptance runs** with `cargo xtask android-run` on both phones: the generated test game
  and `群战示范工程` as before (frames identical to the desktop), Crysalis
  (`/home/doremy/Desktop/rgss/games/ace-crysalis`, RTP `/home/doremy/Desktop/rgss/ref/ace/rtp`,
  built-in scripts `/home/doremy/Desktop/rgss/ref/ace/rpg-rgss3.rb`, frames 60, 210, 360 with
  the input of `ref/tools/oracle/measurements/ace-crysalis-input.txt` if `vkrgss.toml`
  can give input, else frames without input compared with the desktop's run without input),
  and a run with a save and a load of a state. Games of the owner are never committed.

## Out of scope

The zip import, the on-screen buttons, confinement beyond seccomp, the low-memory killer's
policy.

## Follow-ups

None yet.
