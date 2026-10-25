# Design

## Context

- `xtask/src/apk.rs` writes the manifest (`android:label="vkrgss"`, no icon), compiles the Java
  activities, links with `aapt2 link --manifest -I android.jar` without resources, then adds
  `classes.dex` and the libraries with `zip`, aligns and signs.
- The owner's assets (outside the repository, given to the session):
  `ic_launcher_{foreground,background,monochrome}.png` (432x432), `mipmap-{mdpi..xxxhdpi}/
  ic_launcher{,_round}.png`, `mipmap-anydpi-v26/ic_launcher{,_round}.xml` (adaptive icon with
  the three layers as `@drawable/...`). Copy them unchanged.

## Decisions

- **Resources.** `crates/vkrgss-android/res/` with `drawable-nodpi/` (the three layers),
  `mipmap-*/` and `mipmap-anydpi-v26/` as supplied. `aapt2 compile --dir res -o work/res.zip`,
  then `aapt2 link ... work/res.zip` (or `-R`), so that `resources.arsc` and the compiled XML
  are in the APK.
- **Manifest.** `android:label="RGSS Player"`, `android:icon="@mipmap/ic_launcher"`,
  `android:roundIcon="@mipmap/ic_launcher_round"` on `<application>`. Activities keep inheriting
  the label unless one sets its own; check what the library activity shows in the recents list.
- **Licence.** The artwork is the owner's, made for this project; `THIRD-PARTY.md` needs no
  entry. Say in `CHANGELOG.md`.
- **Check on a phone.** Install the debug APK on the Pixel 7a (serial 3A021JEHN02756; free to
  use; the release-signed app from v0.4.0 may be installed there: a debug build cannot update
  it — uninstall only a debug build you installed yourself, never the release one, and if the
  release one is installed use the Galaxy S24+ R5CY21Y3VEV or report) and take a screenshot of
  the launcher/app drawer showing the icon and label (`adb shell screencap`), saved under
  `local/`.

## Out of scope

A Play Store listing graphic, a splash screen, renaming the package or the desktop programs.

## Follow-ups
