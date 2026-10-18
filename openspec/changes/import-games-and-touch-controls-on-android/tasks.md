# Tasks

## 1. Java activity and APK

- [x] 1.1 `VkrgssActivity`, the picker call, JNI, `cargo xtask apk` with `classes.dex`. Verify: the APK installs and starts on both phones; the picker opens from the library screen.

## 2. Import and library

- [ ] 2.1 Unpacking, the rules, the library screen, the RTP import. Verify: scenarios "Import a zip" and "A path that leaves the directory" (unit tests and on both phones through an intent extra).

## 3. Touch and gamepad

- [ ] 3.1 The overlay, multi-touch mapping, gamepad hiding. Verify: scenarios "Buttons move the player" and "Gamepad hides the buttons" on both phones.

## 4. Documents

- [ ] 4.1 `docs/running.md` (Android: importing, buttons), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate import-games-and-touch-controls-on-android --strict` passes.
