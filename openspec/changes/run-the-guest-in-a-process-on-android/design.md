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

## Implementation

- **One relay, compiled twice.** `vkrgss-android` compiles `crates/vkrgss/src/process.rs`
  as a module of its own (`#[path]`, on Android only), as the binding's source is compiled
  twice: the record, the socket, the relay and the states are the desktop's code. What
  differs on Android is behind `cfg(target_os = "android")` in that file: the file names
  of the guest executables (`libvkrgss_guest_rgss12.so`, `libvkrgss_guest_rgss3.so`), the
  program's own file, which is the app's library as `dladdr` names it (the dynamic linker
  loaded it from `nativeLibraryDir`, so its directory is that directory; simpler than JNI
  and as reliable as `/proc/self/maps`), and the message for a missing executable.
  `GuestProcess::start` takes the directory in which it makes the socket's directory: on
  Linux `XDG_RUNTIME_DIR` or the temporary directory as before, on Android the app's
  private files directory (`/data/user/0/<package>/files`), because the temporary
  directory of bionic (`/data/local/tmp`) is not the app's and the external files
  directory cannot hold a socket.
- **The executables in the APK.** `cargo xtask apk` builds `vkrgss-android`,
  `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3` in one cargo run, strips the debug
  information of all three and puts them, compressed, under `lib/arm64-v8a/`; the manifest
  sets `android:extractNativeLibs="true"`. The installer extracts them with mode 0755, and
  `execve` of them from the app works under the app's SELinux domain on both phones: no
  fallback to `fork` without `exec` was needed. The guest executables need only `libc`,
  `libm`, `libdl` and `libz` of the system.
- **The app's own process.** Every game goes through a guest process, so the thread path
  and the refusal of RGSS3 are gone. The app still calls `guest_inputs` of
  `vkrgss-guest-main` before it starts the guest, as `vkrgss` does, so that a game that
  lacks something fails before the window and the process exist, and takes the frame size
  from `vkrgss-binding`; it never starts the interpreter, and the linker leaves it out:
  the app's library has no `ruby_init` and no `rb_*` symbol.
- **The guest's level and log.** The guest is handed the level info, the app's own; it
  inherits the error stream of the app, which leads to logcat and `files/vkrgss.log`, so
  its lines arrive there with the app's.
- **A freshly installed app.** A files directory that `adb shell mkdir` makes is not the
  app's, which then cannot read the game in it (found with a fresh install: the debug key
  of this clone differs from the one the phones had, so the app was uninstalled first).
  `cargo xtask android-run` starts the app once when that directory is missing, so that
  the app makes it, and no longer makes it itself.
- **Acceptance of task 1.1**, 2026-10-17. The games of the change `run-a-game-on-android`
  (`demo`, the game `demo` of the end-to-end tests, frames 1 and 8; `群战示范工程` with the
  RTP of `ref/app`, input `60:RIGHT, 100:, 310:B, 312:`, frames 4, 16, 40 and 90; not
  committed) with `cargo xtask android-run` on the Pixel 7a and the Galaxy S24+, against
  `vkrgss <copy> --guest thread --headless` on this machine (llvmpipe) with the same
  `vkrgss.toml`: all six frames of each phone are identical byte for byte to this
  machine's. The log of each run says `guest process <pid> attached`, that the guest
  filters its system calls with seccomp and that the kernel has no Landlock; after each
  run no process of the app is left.

## Out of scope

The zip import, the on-screen buttons, confinement beyond seccomp, the low-memory killer's
policy.

## Follow-ups

None yet.
