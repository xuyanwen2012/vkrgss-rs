# Bundle a Vulkan loader for macOS 14

## Why

`vkrgss.app` (change `package-a-macos-app`) declares macOS 14 as its minimum in `Info.plist`,
but the Vulkan loader it copies from Homebrew (`vulkan-loader` 1.4.363.0) is built for the
macOS of the machine: `otool -l /opt/homebrew/lib/libvulkan.1.dylib` gives `minos 27.0`. On any
Mac older than macOS 27 the app cannot load it. MoltenVK from Homebrew is built for 12.0 and is
fine; the executables of the workspace are built for 11.0.

## What Changes

- `cargo xtask macos-loader` (on macOS) builds the Khronos Vulkan loader from its source at the
  tag `vulkan-sdk-1.4.363.0` (the version Homebrew has), with
  `CMAKE_OSX_DEPLOYMENT_TARGET=14.0`, against the Vulkan headers of the same tag, into
  `target/macos-loader/`. The source is fetched by git at that tag and its commit is checked
  against a pinned hash.
- `cargo xtask app` copies that loader into the bundle instead of Homebrew's, and builds it first
  when it is missing.
- `cargo xtask app` checks every Mach-O file of the bundle (`LC_BUILD_VERSION`, or
  `LC_VERSION_MIN_MACOSX`) and fails if one needs a newer macOS than the minimum of
  `Info.plist`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: the app runs on macOS 14 and later.

## Impact

`xtask` (`macos-loader`, `app`), `docs/building.md`, `THIRD-PARTY.md` (the loader's licence
stays Apache-2.0), `CHANGELOG.md`. No change of code that runs on Linux or Android.
