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
- **Overlay.** `vkrgss-vk` gained `VkCompositor::set_overlay` and an `OverlayImage`, a
  premultiplied picture of the window's size that the presentation pass blends over the
  frame through a pipeline of its own (`shaders/overlay.frag`); readbacks never draw it.
  The library screen is such a picture, drawn by the app on the CPU (`canvas.rs`: rounded
  rectangles, triangles and text of `vkrgss-text` in `/system/fonts`).
- **Outside the scope:** `docs/building.md` said that the APK has no Java code; its
  paragraph on the app now names the Java class and how it is compiled.

## Out of scope

Release signing, Obtainium metadata, a settings screen, remapping buttons, cloud saves.

## Follow-ups

- **Decided by the owner's session on 2026-10-17:** the hook in
  `crates/vkrgss-host/src/windowed.rs` (a trait or callback in `WindowedOptions`, its
  dispatch, applying what it asks, refreshing the overlay) is in scope; keep it small, keep
  the desktop behaviour unchanged, and test it on the desktop too. Bring back the buttons
  and the gamepad mapping, with the fix that the arrow keys of a gamepad's directional pad
  hide the buttons as well. Also in scope now: a `__MACOSX` directory at the top of a zip is
  ignored by the rule of one directory; the library screen keeps clear of the window's
  insets (navigation bar).

- A game activity that the system destroys while the game runs (an `am start` of the
  library, whose `singleTask` clears the activities above it, or memory pressure) leaves
  the game's process running without a window: winit does not pass `onDestroy` on. The
  process should then end the game.
- winit's Android backend delivers no user event while the activity is paused, so the end
  of an import is shown when the library comes back.
- A zip made by macOS holds a `__MACOSX` directory beside the game's folder, so the rule
  of one directory at the top does not apply and the import finds no `Game.ini`.
- The built-in scripts file still comes from the external files directory, which a player
  without adb cannot fill; an import of it is not part of this change.
- The library screen ignores the window's insets: the three-button navigation bar of the
  Galaxy S24+ lies over the right edge, beside Delete.
- The library screen cannot be driven by a gamepad or a keyboard.
