# Design

## Context

- `crates/vkrgss-android` (changes `run-a-game-on-android`, `run-the-guest-in-a-process-on-android`):
  `android_main`, options from `vkrgss.toml`, logging to logcat, the guest in a process.
- `cargo xtask apk` assembles the APK with aapt2, zipalign and apksigner; JDK 25 and the SDK
  (build-tools 36 with `d8`, platform android-34) are on this machine. Phones on USB, free
  to use for the test APK; `adb shell input tap/swipe` and `uiautomator`-free key events
  can drive tests; `am start -a ... --eu` can pass a URI for a test of the import path
  without the picker UI.

## Decisions

- **Java.** One class, `io.github.xuyanwen2012.vkrgss.VkrgssActivity extends
  android.app.NativeActivity`, with a native method to receive a picked URI and a method the
  native side calls to open the picker. The manifest names it as the activity;
  `hasCode="true"`. Compiled with `javac --release 17 -cp android.jar` and `d8 --min-api 34`.
- **Import.** The native side reads the URI's bytes through JNI
  (`ContentResolver.openInputStream`) into a temporary file in the cache directory, then
  unpacks with the `zip` crate if it is in `Cargo.lock` or the local registry (no network),
  else with a small reader of stored and deflated entries over `miniz_oxide`/`flate2` if
  available; say which. Path traversal and absolute paths are refused; symlinks are not
  created.
- **Overlay.** Drawn by the presenter after the game picture, from a fixed set of
  translucent rounded rectangles and glyphs, scaled with the window; it never changes the
  game's own frame (dumped frames stay identical to the desktop's). Touch events of winit
  (`Touch` with phases) map to pressed keys; several fingers at once.
- **Library screen.** Lists the imported games by title (`Game.ini`'s `Title`), with
  "Import", "Import RTP" and "Delete"; uses the same drawing as the overlay and the
  runtime's font stack for titles.
- **Tests.** Unit tests of the zip rules (one directory level, traversal refused, encodings
  of names) and of the touch mapping; on both phones: import of `群战示范工程` packed as a zip
  through an intent extra (bypassing the picker UI), start, a few taps of the buttons that
  move the player (compare a dumped frame with the desktop's run with the same key
  schedule), and the overlay hidden with a gamepad (if no gamepad, simulate its key events
  with `adb shell input keyevent --source gamepad`).

## Out of scope

Release signing, Obtainium metadata, a settings screen, remapping buttons, cloud saves.

## Follow-ups

None yet.
