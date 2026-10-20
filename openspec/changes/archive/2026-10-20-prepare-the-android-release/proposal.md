# Prepare the Android release

## Why

The Android app (P3) runs every game of the comparison set on the owner's two phones with frames
and saves identical to x86_64 (change `make-the-platform-runs-comparable`). What keeps it from a
release (`docs/decisions.md`, Android: an APK on the GitHub releases, installable through
Obtainium, self-signed) and from the player's hands:

- The APK is signed with a debug key made on each machine, so an update from another build is
  refused (on 2026-10-18 the app had to be uninstalled, losing its data).
- The release workflow builds only the Linux package.
- A player without adb cannot give the app the built-in scripts (`rpg_builtin*.rb`, extracted
  from the original runtime, never redistributed); they are read from the external files
  directory only.
- Sound goes on while the app is in the background.
- On the Pixel 7a, which offers `VK_EXT_present_timing`, frames are paced by the timer.
- The app never ran under the validation layer on a phone.

## What Changes

- `cargo xtask apk --release`: a release build signed with a keystore given by environment
  variables (`VKRGSS_ANDROID_KEYSTORE`, `..._KEYSTORE_PASSWORD`, `..._KEY_ALIAS`,
  `..._KEY_PASSWORD`); without them it refuses. `versionCode` and `versionName` come from the
  workspace version. The debug build stays as it is.
- The release workflow, for a version tag, builds the release APK when the signing secrets are
  present and attaches `vkrgss-<version>-android-arm64.apk` (and its SHA-256) to the release;
  without the secrets it skips the APK with a notice. The owner creates the keystore and the
  repository secrets; `docs/building.md` says how. Obtainium needs nothing beyond a GitHub
  release with that asset; `docs/running.md` says how to add the app there.
- The library screen gets "Import built-in scripts": a file picked with the picker is copied to
  the app's private storage under the name the runtime expects (`rpg_builtin.rb`,
  `rpg_builtin_rgss2.rb`, `rpg_builtin_rgss3.rb`, chosen from the file's content or asked).
  The external files directory keeps working for tests.
- Audio output pauses when the activity is suspended and resumes with it.
- Present timing on the Pixel 7a: find why the pacer falls back to the timer and fix it, or
  record why the surface does not offer it.
- `cargo xtask android-run --validation` packages the Khronos validation layer for Android
  (`libVkLayer_khronos_validation.so`, arm64) into a debug APK and enables it; a run of the
  comparison games on both phones reports no validation error.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: a release APK and the player-side pieces of the Android app.

## Impact

`xtask` (`apk`, `android-run`), `.github/workflows/release.yml`, `crates/vkrgss-android`
(import of built-in scripts, lifecycle of audio), `crates/vkrgss-audio` if pausing needs it,
`crates/vkrgss-vk` (present timing on Android, if the cause is there), `docs/building.md`,
`docs/running.md`, `CHANGELOG.md`.
