# Tasks

## 1. Guest programs in the APK

- [ ] 1.1 The two guest programs under `lib/arm64-v8a/`, `extractNativeLibs`, and starting a guest from `nativeLibraryDir`. Verify: a game of RGSS1 runs on both phones with the guest in a process (frames identical to the desktop).

## 2. VX Ace and states

- [ ] 2.1 Games of RGSS3, save states, the end of the app. Verify: scenarios "A game of VX Ace on a phone", "Save and load a state" and "The app ends" on both phones.

## 3. Documents

- [ ] 3.1 `docs/running.md` (Android), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-the-guest-in-a-process-on-android --strict` passes.
