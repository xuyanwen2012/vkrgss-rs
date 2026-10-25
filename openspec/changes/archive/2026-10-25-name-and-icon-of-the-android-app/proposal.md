# Name and icon of the Android app

## Why

The Android app shows the label `vkrgss` and the system's default icon on the home screen.
The owner chose the name "RGSS Player" and supplied a launcher icon (a pixel-art ruby on dark
plum) on 2026-10-25.

## What Changes

- The app's label is `RGSS Player`. The package name `io.github.xuyanwen2012.vkrgss` stays, so
  installs and Obtainium updates are unaffected.
- The APK carries the owner's launcher icon: an adaptive icon (foreground, background and a
  monochrome layer for themed icons, as PNGs) for Android 8 and later, and the legacy and
  round PNGs per density. The manifest names `@mipmap/ic_launcher` and
  `@mipmap/ic_launcher_round`.
- `cargo xtask apk` compiles the resources with `aapt2 compile` and links them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: the name and the icon of the Android app.

## Impact

`xtask/src/apk.rs`, new resources under `crates/vkrgss-android/res/`, `docs/building.md`,
`docs/running.md` where the app is named, `README.md` if it names the app, `CHANGELOG.md`.
