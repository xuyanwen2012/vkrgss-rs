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

## As built

What the implementation decided where the decisions above leave a choice, or where it had to
depart from them, and why.

- **Two activities, one class.** The library is `VkrgssActivity`; a game runs in the nested
  class `VkrgssActivity$Game`, declared with `android:process=":game"`. winit's event loop
  can be built once per process, and a session takes its options (key map, sound, frame
  limit, dumps, input script) when it starts, so the library and a game cannot share one
  process; the game's process ends with the game as before, and the library comes back. The
  game's directory is the intent's extra `game`; without it the game activity runs the
  directory `game` of the external files directory, which keeps `android-run` working.
- **The copy of the picked zip.** `copyToCache` of the Java class reads the URI through
  `ContentResolver.openInputStream` into `cache/import.zip`; the native import thread calls
  it through JNI (the `jni` crate, already in `Cargo.lock` through `android-activity`) and
  then unpacks. Finishing the activity also goes through JNI (`finish`), which replaces the
  call of `ANativeActivity_finish` with the pointer of `activity_as_ptr`, a JNI reference
  and not an `ANativeActivity`.
- **javac.** The JDK of this machine has no `javac` executable and no `ct.sym`, so
  `--release 17` is not available; `cargo xtask apk` runs the compiler module of the JDK
  (`java -m jdk.compiler/com.sun.tools.javac.Main`) with `-source 8 -target 8
  -bootclasspath android.jar`, which compiles against Android's classes as `--release`
  could not, and `d8 --min-api 34`.
- **Zip.** There is no `zip` crate offline, so `unpack.rs` is a small reader of stored and
  deflated entries over `flate2`, with Zip64 sizes and a check of every CRC-32. A directory
  `__MACOSX` at the top is left out and does not count for the rule of one directory.
- **The run-time package's name** is told by the folders of `Graphics` that only one
  runtime's package has: `Autotiles` (XP, `Standard`), `Battlebacks1` (VX Ace, `RPGVXAce`),
  else `System` (VX, `RPGVX`). Every game gets the imported packages by these names.
- **"The last one played starts first"** is read as: listed first and selected, so that one
  tap starts it.
- **Overlay.** `vkrgss-vk` gained `VkCompositor::set_overlay` and an `OverlayImage`, a
  premultiplied picture of the window's size that the presentation pass blends over the
  frame through a pipeline of its own (`shaders/overlay.frag`); readbacks never draw it.
  The app draws the picture on the CPU (`canvas.rs`: rounded rectangles, triangles and text
  of `vkrgss-text` in `/system/fonts`), for the buttons and for the library screen alike.
  The buttons of `Input` are held as a gamepad's (`pad_buttons` of the session), which
  needs no key of a runtime's key map; F3 and F4 are pressed as keys, so that the host's
  states answer them; the menu button asks the game to quit when the finger lifts on it.
- **The hook of the windowed runner** (`crates/vkrgss-host`, allowed by the decision under
  Follow-ups): the trait `Overlay` (`overlay.rs`) gets the window's touches and keys and
  answers with `OverlayAction`s (hold buttons, press a key, quit), and gives its picture when
  it has changed; `WindowedOptions::overlay` holds one. `Driver::overlay` applies the
  actions and `Driver::pad_buttons` holds the overlay's buttons with the pads', so that the
  application is tested on the desktop with the driver's other tests; the runner only
  dispatches the events and hands the picture to the compositor. Touches reach nothing
  without an overlay, so the desktop runs as before; with one, a touch also answers a
  message panel, as a mouse press does.
- **Gamepads.** winit's Android backend gives `KEYCODE_BUTTON_*` as unidentified keys, which
  the buttons map with the desktop's `GamepadMap`; a gamepad's directional pad arrives as
  the arrow keys, which the key map holds and which hide the buttons too. Android's key
  events say nothing of their device here, so a keyboard's arrow keys hide them as well.
- **Insets.** The target SDK makes the window reach under the system's bars, so the Java
  class keeps the insets of the system's bars and the display's cutout from the decor
  view's `OnApplyWindowInsetsListener`, and the native side reads them (`insets`) whenever
  it draws: the library screen and the buttons keep clear of them, also when the bar
  changes sides with a turn of the phone, which does not resize the window.
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
  insets (navigation bar). Done as described under "As built".

- A game activity that the system destroys while the game runs (an `am start` of the
  library, whose `singleTask` clears the activities above it, or memory pressure) leaves
  the game's process running without a window: winit does not pass `onDestroy` on. The
  process should then end the game.
- winit's Android backend delivers no user event while the activity is paused, so the end
  of an import is shown when the library comes back.
- The built-in scripts file still comes from the external files directory, which a player
  without adb cannot fill; an import of it is not part of this change.
- The on-screen buttons show no pressed state, and the library screen cannot be driven by
  a gamepad or a keyboard.
- On the Pixel 7a the game's process sometimes dies of a SIGSEGV in ART's `Jit thread pool`
  after the game has ended ("the app ends" is logged, the library comes back); it happens
  also with the APK before this change's buttons (once in three runs), so it belongs to the
  end of the process, which exits while ART's threads run.
