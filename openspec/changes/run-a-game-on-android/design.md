# Design

## Context

- Tools on this machine: NDK r30 at `/home/doremy/android-ndk-r30`; SDK at
  `/home/doremy/Android/Sdk` (build-tools 33-36, platforms android-34 and android-37.0,
  cmdline-tools, platform-tools); JDK 25. No emulator. Phones on USB, free to use (install
  and uninstall the test APK freely, nothing else): Pixel 7a `3A021JEHN02756` (Mali-G710,
  Vulkan 1.4, has `VK_EXT_present_timing`), Galaxy S24+ `R5CY21Y3VEV` (Xclipse 940, Vulkan
  1.3, no present timing).
- The adb test runner of `build-the-interpreters-for-android` (`cargo xtask android-test`)
  shows how to cross-build and push; `docs/building.md` has the Android section.
- winit 0.30 supports Android with the feature `android-native-activity`; the app's entry is
  `android_main(app: AndroidApp)`.

## Decisions

- **NativeActivity, no Java.** The manifest names `android.app.NativeActivity` with
  `android.app.lib_name` = the cdylib; `android:hasCode="false"`; minSdk 34, targetSdk 37;
  landscape, fullscreen. The APK is assembled by `cargo xtask apk` with aapt2 (link the
  manifest against `platforms/android-34/android.jar`), the `.so` under
  `lib/arm64-v8a/`, zipalign and apksigner with a debug keystore generated under
  `target/` on first use. No Gradle.
- **Guest on a thread.** The Android package links Ruby 1.8.7 and runs RGSS1 and RGSS2 games
  with the guest on a thread, as `vkrgss --guest thread` does. A game of RGSS3 is refused
  with a message for now (it needs Ruby 1.9.2 in a process). Save states are off.
- **Surface.** `ash-window` creates the surface from the window's raw handles. On
  `Suspended` the swapchain and surface are destroyed and the game waits; on `Resumed` they
  are created again. The swapchain uses the surface's `currentTransform` as
  `preTransform` and the presenter rotates the scaled picture accordingly, or, simpler and
  acceptable for the first version, `IDENTITY` and lets the compositor rotate (state which,
  and why).
- **Options.** `vkrgss.toml` in the game directory, read as the layered configuration is on
  Linux, gives `frames` and `dump` entries for tests; logs go to logcat
  (`android_logger` or `tracing` with a logcat writer) and to a file in the files dir.
- **Acceptance run.** `cargo xtask android-run` on both phones with the generated test game
  that the end-to-end tests of `vkrgss` use, and with `群战示范工程` of the owner's games
  (`/home/doremy/Desktop/rgss/games/群战示范工程`, never committed; frames 4,16,40,90 with the
  input of `ref/tools/oracle/regress.sh` if input can be scheduled through `vkrgss.toml`,
  else without input), the RTP from `/home/doremy/Desktop/rgss/ref/app`. The dumped frames
  are compared with those of `vkrgss --guest thread --headless --dump` on this machine:
  they must be identical, because composition is the same; record the result in
  `design.md`.

## Implementation

- **Rotation: `IDENTITY`, the compositor rotates.** A swapchain is created with the
  transform `IDENTITY` wherever the surface lists it, else with the surface's current one
  (the desktop surfaces report `IDENTITY` as current, so nothing changes there). Both phones
  report the surface of the landscape window as 2400 by 1080 (Pixel 7a) and 2340 by 1080
  (Galaxy S24+) with the current transform `ROTATE_90`, and list `IDENTITY`. Chosen because
  the presenter's letterbox, filters and mask are computed in window pixels, and rotating
  them in `present.frag` would touch every one of them for a picture of 640 by 480 that the
  compositor turns at little cost. Android calls every present of such a swapchain
  `VK_SUBOPTIMAL_KHR`; the swapchain remembers the transform it was created under, and a
  suboptimal present replaces it only when the surface's transform or extent has changed
  (a turn to the other landscape side does that). Measured: one swapchain per surface on
  both phones, where without this it was replaced at every frame.
- **Surface loss.** `VkCompositor::release_surface` destroys the swapchain and the surface
  and keeps the device, the bitmaps and the frames; `present` shows nothing until
  `attach_window` gives it the surface of the window that came back, which the device's
  queue must be able to present to. The windowed runner of `vkrgss-host` does this on
  winit's `Suspended` and `Resumed`, which desktop window systems never send, and sends no
  tick while the window is away, so the game waits. A frame limit and frame dumps were
  added to the windowed runner (`frames`, `dumps` of `WindowedOptions`, unset on the
  desktop), because a phone has no headless surface.
- **Pause and resume (task 2.1).** The demo game of the end-to-end tests, looping for 2400
  frames, on both phones: `KEYCODE_HOME`, then `am start` again; on the Pixel also the
  screen going off and on. The log shows the surface going and coming back with one new
  swapchain each time, a screenshot after the resume shows the game upright and moving,
  the game runs to its end and the app ends normally. No Khronos validation layer for
  Android is on this machine (no network), so validation on the phones was not run; the
  same release and attach path runs under the validation layer on lavapipe with a headless
  surface (`a_released_surface_shows_nothing_until_a_new_one_is_attached`), without an
  error.
- **Options on the phone.** `vkrgss.toml` of the game is read with the key names, the
  formats and the relative paths of the Linux file, as its only layer; it takes `rtp`,
  `builtin_scripts`, `font_dirs`, `audio`, `soundfont`, `seed`, `debug`, `log_p`, `frames`,
  `input` and `dump`, and any other key is an error that names it. The input script holds
  the buttons frame by frame as in a headless run (an `InputSource` of the app). Without
  `builtin_scripts` the file of the runtime in the files directory is read; the system's
  fonts (`/system/fonts`) are searched after `font_dirs`. The log goes to logcat (tag
  `vkrgss`) and to `files/vkrgss.log` through a pipe that replaces the standard output and
  the error stream, so script errors arrive too. When the game has ended the activity is
  finished, its destruction awaited, and the process exits: the interpreter cannot start
  twice in one process.
- **Game of RGSS3.** The app reads `Game.ini` before anything else of the game; for RGSS3
  it writes "a game of RPG Maker VX Ace (rgss3) is not supported on Android yet: ..." to
  logcat and the log file and ends, with the activity finished. "Says" is that log line
  in this version, since the app has no other text on screen before a game runs.
- **Acceptance (task 3.1), 2026-10-17, at commit 12339844 plus the files of this task.**
  The games were assembled under `local/games/` (not committed): `demo`, the game `demo`
  of `crates/vkrgss/tests/end_to_end.rs` (its three scripts, the stand-in built-in
  scripts, Liberation Sans from `vkrgss-text/tests/fonts`, input `2:RIGHT, 6:`, frames 1
  and 8, no audio), and `群战示范工程` with the RTP of `ref/app`, Noto Sans CJK SC and
  `ref/rpg_builtin.rb` in the game directory, seed 0, input `60:RIGHT, 100:, 310:B, 312:`
  of `ref/tools/oracle/regress.sh` through `vkrgss.toml`, frames 4, 16, 40 and 90, a
  limit of 91 frames, with audio. `cargo xtask android-run <SERIAL> <GAME>` on the Pixel
  7a (Mali-G710) and the Galaxy S24+ (Xclipse 940) against `vkrgss <copy> --guest thread
  --headless` on this machine (llvmpipe) with the same file: all six frames of each phone
  are identical byte for byte to those of this machine. AAudio opened a stream of 48 kHz
  for `群战示范工程` without an error. A game whose `Game.ini` names `RGSS301.dll` and
  `Scripts.rvdata2` is refused on both phones with the message above, and the process
  ends without being started again. No game of RGSS2 was run on a phone.
- **Commits.** The first commit holds the code of the surface and the session as well,
  because the app links it; the later ones hold what their tasks changed after it.

## Out of scope

The guest in a process on Android, RGSS3 on Android, the zip import, on-screen buttons,
gamepads beyond what winit delivers, release signing, Obtainium metadata.

## Follow-ups

None yet.
