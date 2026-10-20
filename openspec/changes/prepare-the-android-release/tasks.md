# Tasks

## 1. Release APK

- [x] 1.1 `apk --release`, the version code, the release workflow step, its test. Verify: scenarios "Signed with the given key" and "No keystore"; `cargo test -p xtask --locked` passes.

## 2. Built-in scripts and audio

- [x] 2.1 Import of built-in scripts, pause of audio. Verify: scenarios "Built-in scripts imported" and "Sound in the background" on both phones (the latter by the audio stream's state in logcat).

## 3. Present timing and validation

- [x] 3.1 The Pixel's present timing; `android-run --validation`. Verify: scenario "Validation on the phones"; the cause of the timer pacing on the Pixel fixed or recorded.

## 4. Documents

- [x] 4.1 `docs/building.md` (release keystore and secrets), `docs/running.md` (Obtainium, importing built-in scripts), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate prepare-the-android-release --strict` passes.
