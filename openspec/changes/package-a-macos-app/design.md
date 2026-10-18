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

## Follow-ups

None yet.
