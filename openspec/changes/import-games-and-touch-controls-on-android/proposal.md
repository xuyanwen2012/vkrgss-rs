# Import games and touch controls on Android

## Why

`docs/decisions.md` ("Platforms", Android, first version): a game is imported as a zip into
the app's private directory, and input comes through on-screen buttons and gamepads. Today
the app (`vkrgss-android`) reads one game from its external files directory, filled with
`adb push`, and takes keys only from a hardware keyboard; a player without adb cannot use it.

## What Changes

- A library of games in the app's private storage: the app shows the imported games and
  starts the chosen one; the last one played starts first.
- Importing: the system file picker (`ACTION_OPEN_DOCUMENT`, type zip) chooses a zip, which
  is unpacked into a directory of its own in private storage; a zip whose top level is one
  directory with `Game.ini` inside is unpacked one level down. Names in the zip are read as
  UTF-8 or, when that fails, as the encoding the game's other text uses (Shift-JIS, GBK)
  following the existing rules of the runtime; paths that leave the target directory are
  refused. The RTP is imported the same way, by a zip that holds the RTP's folders.
- The choice of game and the import are drawn by the app itself in Vulkan (a list and a few
  buttons), so no layout of Android views is needed.
- On-screen buttons over the game: a direction pad, C (confirm), B (cancel), A, X, Y, Z, L, R,
  F3, F4 (states) and a menu button, drawn translucent at the sides, with multi-touch;
  they press the keys the runtime already maps. Hidden while a gamepad is connected.
- Gamepads: the key events of winit's Android backend mapped as on the desktop.
- To get the result of the picker, a small Java class extends `NativeActivity`
  (`onActivityResult`) and passes the chosen URI to the native code through JNI; it is
  compiled with `javac` and `d8` of the SDK by `cargo xtask apk` (no Gradle).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: a player imports games and plays them with touch or a gamepad.

## Impact

`crates/vkrgss-android` (library, import, touch overlay, JNI), a small Java source under it,
`xtask` (`apk`: Java compiled into `classes.dex`; `android-run` keeps working with the files
directory as a game source for tests), `crates/vkrgss-vk` only if the overlay needs a
drawing path the presenter lacks, `docs/running.md`, `CHANGELOG.md`.
