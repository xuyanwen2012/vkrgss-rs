# Tasks

## 1. Resources and manifest

- [x] 1.1 The resources, `aapt2 compile` and link, the label and icons in the manifest. Verify: scenario "Name and icon" by unit test of the manifest and by `aapt2 dump badging` of a built APK (label `RGSS Player`, icon resources present); the existing apk tests pass.

## 2. On a phone

- [x] 2.1 Install the debug APK on a phone as design.md says and screenshot the launcher. Verify: the screenshot shows the ruby icon and "RGSS Player"; the app still starts its library screen.

## 3. Documents

- [ ] 3.1 `docs/building.md`, `docs/running.md`, `README.md` where the app is named, `CHANGELOG.md`. Verify: the document tests of `xtask` pass.

## 4. Checks

- [ ] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0; `openspec validate name-and-icon-of-the-android-app --strict` passes.
