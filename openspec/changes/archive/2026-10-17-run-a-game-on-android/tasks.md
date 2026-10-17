# Tasks

## 1. Package and APK

- [x] 1.1 `vkrgss-android`, `cargo xtask apk`, the manifest and the debug key. Verify: the APK installs on both phones and its activity starts (logcat shows the app's first log line).

## 2. Presentation and lifecycle

- [x] 2.1 Android surface, swapchain, suspend and resume, rotation. Verify: scenario "Pause and resume" on both phones (`adb shell input keyevent KEYCODE_HOME`, then `am start` again).

## 3. Game, options, audio

- [x] 3.1 The game from the files directory, `vkrgss.toml`, the guest on a thread, audio, the refusal of RGSS3, `cargo xtask android-run`. Verify: scenarios "Frames on a phone" and "Game of RGSS3".

## 4. Documents

- [x] 4.1 `docs/building.md`, `docs/running.md` (Android), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Integration check

- [x] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-a-game-on-android --strict` passes.
