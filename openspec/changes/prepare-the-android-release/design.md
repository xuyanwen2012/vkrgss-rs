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
  paused, so music continues where it stopped. The events arrive in the windowed runner of
  `vkrgss-host`, outside the scope of this change: `AudioSink` gained `set_paused` (a default
  that does nothing, and `Output`'s), `Session` `set_audio_paused`, and the runner's
  `suspend` and `resume` call it, a few lines that the requirement plainly needs.
  `Output::set_paused` of `vkrgss-audio` pauses the `cpal` stream (AAudio's
  `requestPause`) and makes its callback give silence without mixing.
- **Present timing.** Read the pacer's log on the Pixel; check whether the swapchain is created
  with the present-timing flags on Android (the window path may differ from the desktop's) and
  whether `vkGetSwapchainTimeDomainPropertiesEXT` returns usable domains. Fix only a defect of
  ours; a driver limitation is recorded.
  Found on 2026-10-20 on the Pixel 7a (Mali-G710, driver r54p3, Android 17
  CP3A.260905.009): the device has every extension and feature of the pacer and the surface
  supports present timing at an absolute time with all four stages, but the swapchain offers
  one time domain, `VK_TIME_DOMAIN_PRESENT_STAGE_LOCAL_EXT`, and
  `vkGetPhysicalDeviceCalibrateableTimeDomainsKHR` lists only `DEVICE`, `CLOCK_MONOTONIC`
  and `CLOCK_MONOTONIC_RAW`. A target time on that clock needs a reading of it, and
  VUID-VkCalibratedTimestampInfoKHR-timeDomain-02354 allows `vkGetCalibratedTimestampsKHR`
  only for a domain the device lists, so the pacer correctly finds no domain it can read and
  presents by the timer. It is the driver's limitation, not a defect of ours; the pacer now
  says so in the log once per swapchain. The Galaxy S24+ (Xclipse 940) lists none of the
  pacing extensions.
- **Validation run.** `android-run --validation` adds the layer to `lib/arm64-v8a/`, sets
  `android:debuggable` (already debug) and enables it through the app's own instance layers
  (the app asks for `VK_LAYER_KHRONOS_validation` when a debug option is set), and routes the
  messenger to logcat; the run fails if a validation error is logged. The layer's file is
  named by `VKRGSS_ANDROID_VALIDATION_LAYER`. The debug option is the extra `validation`
  of the intent of the activity of games, which `android-run --validation` gives `am
  start`; the app passes a `ValidationLog` to the compositor and, once the window has gone,
  writes each error it holds to the log (`validation error: ...`) and whether there was
  none. The windowed runner of `vkrgss-host` (outside the scope) gained
  `WindowedOptions::validation`, which it hands to `VkCompositor::for_window`: three lines
  that the requirement plainly needs, since the runner makes the game's compositor. The
  scenario runs "the comparison games", which `xarch android` runs; it takes `--validation`
  too and passes it to the same install and start as `android-run`.

  Run on 2026-10-20 with the layer of vulkan-sdk-1.4.363.0: `cargo xtask xarch android
  --validation` with the ten games of `local/xarch.toml` (qunzhan, mbbs, mh, zhanshen,
  tower, xinbing, arcis, vv, crysalis, isoi) exited 0 on the Pixel 7a and on the Galaxy
  S24+; every game's logcat says `validation: the layer reported no error`, and the loader
  logs `Loaded layer VK_LAYER_KHRONOS_validation` from the app's native library directory.
  `android-run --validation` with a copy of qunzhan passed on the Pixel 7a too.

## Out of scope

Google Play, F-Droid, a release keystore made by the flow, iOS.

## Follow-ups

- Present timing on the Mali driver of the Pixel 7a: a target time could be given relative
  to the previous present (`presentAtRelativeTime`) instead of on a clock that has to be
  read, or ARM could list the stage's domain as calibrateable; neither is built here.
- The library screen does not run under the validation layer; only games do.
- No release with an APK exists yet, so Obtainium was not tried against a real release; the
  release workflow's Android step runs for the first time with the owner's secrets.
