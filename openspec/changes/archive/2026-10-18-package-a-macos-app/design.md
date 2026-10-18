# Design

## Context

- `crates/vkrgss-vk/src/device/instance.rs:61` calls `ash::Entry::load()`; tests use it too.
- On the Mac: Homebrew's `vulkan-loader` (`/opt/homebrew/lib/libvulkan.1.dylib`) and
  `molten-vk` (`/opt/homebrew/lib/libMoltenVK.dylib`, ICD
  `/opt/homebrew/etc/vulkan/icd.d/MoltenVK_icd.json`). `docs/building.md` "## macOS" says
  how the workspace is built and tested there.
- The guest programs are separate executables (`vkrgss-guest-rgss12`, `vkrgss-guest-rgss3`)
  found beside `vkrgss`, so they live in `Contents/MacOS/` too.

## Decisions

- **Loading.** `ash::Entry::load_from(path)` with the bundle's loader when
  `current_exe()` is `<x>.app/Contents/MacOS/<name>`; set `VK_DRIVER_FILES` to the bundle's
  ICD before loading if neither variable is set. The ICD JSON's `library_path` is
  `../../../Frameworks/libMoltenVK.dylib` (relative to the JSON, as the loader allows).
- **Install names.** `install_name_tool -id @rpath/libvulkan.1.dylib` and
  `-id @rpath/libMoltenVK.dylib`; the executables get `-add_rpath @executable_path/../Frameworks`
  (only if they link dylibs dynamically; `ash` loads the loader with `dlopen` and needs the
  path, not the rpath). Check with `otool -L`.
- **Folder chooser.** `osascript` returns a POSIX path with `POSIX path of (choose folder
  with prompt "...")`; a cancel ends the app quietly. The last directory is kept in the
  configuration directory (`~/Library/Application Support/vkrgss/` through the existing
  configuration layer, or `dirs`), and offered as the default location of the chooser.
- **Signing.** Ad-hoc only. Because a locally built app carries no quarantine attribute, it
  opens with a double-click on the machine that built it; a downloaded copy needs
  `xattr -d com.apple.quarantine` until the Developer ID signature comes; say so in
  `docs/running.md`.
- **Acceptance on the Mac.** `cargo xtask app` succeeds; `open target/app/vkrgss.app --args
  <generated game>` is not checkable over ssh with a window, so the check is headless:
  `target/app/vkrgss.app/Contents/MacOS/vkrgss --headless --frames 60 --dump 30=<png>
  <generated game>` run from a shell with no `DYLD_*` variables and no Homebrew in `PATH`
  writes the frame, identical to the one of the development build; and `otool -L` of every
  binary in the bundle names nothing under `/opt/homebrew`. A windowed run (a window
  appears, a game plays, sound is heard) is checked by the owner at the Mac and recorded in
  `design.md` when done; the flow does not wait for it.

## Out of scope

Developer ID signing, notarization, a DMG, Sparkle updates, a menu bar, drag and drop of a
game folder onto the icon.

## Notes from the implementation

- **Icon.** None: the README banner is a wide JPEG with text, and an icon cut from it would
  be a design of its own, not the simple case the proposal allows.
- **Rpath.** The three executables link only libraries of the system (`otool -L`), so they
  get no rpath; `vkrgss-vk` loads the loader of the bundle by its path.
- **Acceptance on the Mac (2026-10-17).** `cargo xtask app` built the app; `otool -L` of the
  three executables and two libraries names nothing under `/opt/homebrew`; the bundle's
  `vkrgss`, run headless from `env -i` with `PATH=/usr/bin:/bin` on a generated game of RGSS1
  with a sprite, a window and text, wrote frame 30 identical byte for byte to that of the
  development build, with the guest on a thread and in a process. The windowed run is the
  owner's to check.
- **Checks on the Mac (2026-10-17).** `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass with the
  environment of `docs/building.md`. Outside task 5.1, `cargo doc --workspace` with warnings
  denied stops on the Mac at an unresolved link to `seccomp` in `vkrgss-guest`, which this
  change does not touch; the crates it changes document without warnings there.

## Follow-ups

- **Minimum macOS.** `Info.plist` declares macOS 14, and the executables are built for 11.0,
  but the loader copied from Homebrew on the owner's Mac is built for macOS 27 (`minos 27.0`
  in `otool -l`; MoltenVK's is 12.0). The app therefore runs only on the macOS that
  Homebrew's bottle was built for. The release needs a loader built for macOS 14, or the
  declared minimum raised.
