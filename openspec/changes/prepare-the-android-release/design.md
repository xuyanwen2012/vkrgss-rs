# Design

## Context

- `xtask/src/apk.rs` assembles the APK and signs it with `target/apk/debug.keystore`.
- The Khronos validation layer for Android, release vulkan-sdk-1.4.363.0, is extracted at
  `../../vkxp/target/android-vvl/extracted/android-binaries-1.4.363.0/arm64-v8a/libVkLayer_khronos_validation.so`
  (read only; downloaded by the owner's session; Apache-2.0). It is never committed.
- Phones on USB, free to use for the test APK: Pixel 7a `3A021JEHN02756`, Galaxy S24+
  `R5CY21Y3VEV`. `cargo xtask xarch android` runs the comparison games (game list in
  `../../vkxp/local/xarch.toml`, read only).
- The release keystore does not exist yet and must not be made by the flow; tests of the
  release path use a throwaway keystore under `target/`.

## Decisions

- **Signing.** `apksigner sign --ks $VKRGSS_ANDROID_KEYSTORE --ks-key-alias ... --ks-pass env:...
  --key-pass env:...` (passwords through `env:` so they never appear in a command line); v2
  and v3 signatures. In CI the keystore is a base64 secret written to a temporary file.
- **Version code.** `major * 1_000_000 + minor * 1_000 + patch` from the workspace version.
- **Built-in scripts import.** Detect the runtime from the content (the class list of RGSS1,
  RGSS2 or RGSS3, e.g. `RPG::Weather` only in RGSS1, `RPG::BaseItem` in RGSS2/3, RGSS3's
  `RPG::BaseItem::Feature`); if unclear, ask with three buttons. Size limit 1 MiB; the file is
  stored as is.
- **Audio.** On `Suspended`, pause the cpal stream (or stop pulling and output silence if
  pausing is not available), on `Resumed` play; the mixer's position does not advance while
  paused, so music continues where it stopped.
- **Present timing.** Read the pacer's log on the Pixel; check whether the swapchain is created
  with the present-timing flags on Android (the window path may differ from the desktop's) and
  whether `vkGetSwapchainTimeDomainPropertiesEXT` returns usable domains. Fix only a defect of
  ours; a driver limitation is recorded.
- **Validation run.** `android-run --validation` adds the layer to `lib/arm64-v8a/`, sets
  `android:debuggable` (already debug) and enables it through the app's own instance layers
  (the app asks for `VK_LAYER_KHRONOS_validation` when a debug option is set), and routes the
  messenger to logcat; the run fails if a validation error is logged.

## Out of scope

Google Play, F-Droid, a release keystore made by the flow, iOS.

## Follow-ups

None yet.
