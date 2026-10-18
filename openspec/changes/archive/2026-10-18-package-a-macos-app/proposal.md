# Package a macOS app

## Why

P2 of `docs/decisions.md` (macOS). The workspace builds and its tests pass on the Mac
(change `build-and-test-on-macos`), but a player cannot run it: `ash` finds no
`libvulkan.dylib` without `DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib`, which macOS strips
from protected programs and which a double-clicked app never has, and MoltenVK must be
installed with Homebrew. A Mac app carries its libraries inside its bundle.

## What Changes

- `vkrgss-vk` on macOS loads the Vulkan loader from the app bundle when the executable lies
  in one (`Contents/Frameworks/libvulkan.1.dylib`), and points the loader at the bundle's
  MoltenVK ICD (`Contents/Resources/vulkan/icd.d/MoltenVK_icd.json`) unless the user set
  `VK_DRIVER_FILES`/`VK_ICD_FILENAMES`; outside a bundle it keeps today's behaviour, with
  `/opt/homebrew/lib` as a last fallback for development.
- `cargo xtask app` (on macOS) builds `vkrgss`, `vkrgss-guest-rgss12` and
  `vkrgss-guest-rgss3` for release and assembles `target/app/vkrgss.app`:
  `Contents/MacOS/` with the three executables, `Contents/Frameworks/` with the loader and
  MoltenVK copied from Homebrew with their install names and the executables' rpaths set to
  the bundle, the ICD JSON with a relative `library_path`, `Info.plist` (bundle identifier
  `io.github.xuyanwen2012.vkrgss`, minimum macOS 14, the app's icon from the README banner if
  simple, else none), the licence texts (MoltenVK's and the loader's Apache-2.0 included),
  and an ad-hoc signature (`codesign --sign - --deep`). A zip of the app beside it.
- Starting the app without a game directory (a double-click) asks for one with the system's
  folder chooser (`osascript -e 'choose folder'`), and remembers the last one in the
  configuration directory; `open vkrgss.app --args <game dir>` and the command line work as
  on Linux.
- Signing with a Developer ID and notarization are not part of this change (they come with
  the release, per `docs/decisions.md`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: a self-contained macOS app.

## Impact

`crates/vkrgss-vk` (loading the loader on macOS), `crates/vkrgss` (asking for a game
directory on macOS), `xtask` (`app`), `docs/building.md`, `docs/running.md`, `THIRD-PARTY.md`,
`CHANGELOG.md`. No change on Linux or Android.
