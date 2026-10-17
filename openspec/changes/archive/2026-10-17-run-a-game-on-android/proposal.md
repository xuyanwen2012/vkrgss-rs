# Run a game on Android

## Why

P3 of `docs/decisions.md`. Both interpreters run on the owner's phones (change
`build-the-interpreters-for-android`), the guest is filtered with seccomp (change
`filter-the-system-calls-of-the-guest`), and the Vulkan backend passes its differential
tests on Mali-G710 and Xclipse 940. What is missing is a program that a phone can start:
an APK that opens a window, presents through `VK_KHR_android_surface`, and runs a game. The
swapchain path was never exercised on a phone (Android has no headless surface).

## What Changes

- A new package `vkrgss-android`: a `cdylib` built as the native library of an APK with
  `NativeActivity` (no Java code), through winit's Android backend (`android-activity`).
  It runs the host as `vkrgss` does in a window, with the guest on a thread (the guest in a
  process of its own on Android is a later change).
- Presentation on Android: the surface of the window through `ash-window`, recreated when
  the activity is paused and resumed (the window goes away and comes back), and the
  pre-rotation the surface reports.
- The game: the first version reads it from the app's external files directory
  (`/sdcard/Android/data/<package>/files/game/`), which `adb push` can fill; the zip import
  and the on-screen buttons are a later change. A file `vkrgss.toml` there may give the
  options of a run (for the tests: a frame limit and frames to dump).
- Audio through cpal's AAudio backend.
- `cargo xtask apk`: builds the debug APK with the NDK and the SDK of this machine
  (aapt2, zipalign, apksigner with a debug key made locally) at minSdk 34.
- `cargo xtask android-run <device> <game>`: installs the APK, pushes the game, starts it,
  waits, pulls the dumped frames and the log, uninstalls nothing (a later run reinstalls).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: a game runs and is presented on Android (P3 not yet accepted).

## Impact

New `crates/vkrgss-android`; `crates/vkrgss-vk` (Android surface, pre-rotation, surface
loss), `crates/vkrgss-host` (Android window lifecycle; the loader-isolation check stays
desktop-only), `crates/vkrgss-audio` if cpal needs a feature, `xtask` (`apk`,
`android-run`), workspace `Cargo.toml` and `Cargo.lock`, `docs/building.md`,
`docs/running.md`, `CHANGELOG.md`.
