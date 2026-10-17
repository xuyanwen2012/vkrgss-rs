# Run the guest in a process on Android

## Why

The Android app (`vkrgss-android`, change `run-a-game-on-android`) runs the guest on a thread
of the app, so it links Ruby 1.8.7 only: games of VX Ace are refused, and save states, which
copy the guest process with `fork`, are off. On the desktop the guest is a process of its
own (`vkrgss-guest-rgss12`, `vkrgss-guest-rgss3`), confined by Landlock and the seccomp
filter. On Android an app targeting API 29 or later cannot execute files of its private data
directory (W^X); it can execute what the package installs under its native library
directory (`nativeLibraryDir`, `lib/arm64-v8a/` of the APK, extracted with
`android:extractNativeLibs="true"`).

## What Changes

- The APK carries both guest programs as `lib/arm64-v8a/libvkrgss_guest_rgss12.so` and
  `libvkrgss_guest_rgss3.so` (the executables under names the installer accepts), and the
  manifest sets `extractNativeLibs`.
- The app starts the guest of a game as a child process from `nativeLibraryDir`, through the
  same record and socket as the desktop's process guest; games of VX Ace run.
- The guest confines itself with the seccomp filter (Landlock is absent on the phones and
  stays best-effort).
- Save states (F3, F4 on a keyboard; the on-screen buttons come later) work as on the
  desktop.
- When the app is killed or ends, the guest and its copies end too (the parent-death signal
  of Linux works on Android).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: on Android the guest runs in a process, games of VX Ace run, and
  save states work.

## Impact

`crates/vkrgss-android`, `crates/vkrgss` (`process.rs`: where the guest executable lies on
Android), `xtask` (`apk`: the two guest programs in the APK), `docs/running.md`,
`CHANGELOG.md`.
