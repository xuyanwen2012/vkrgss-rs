# Design

## Context

- `xtask/src/oracle/original.rs` prepares the prefix: registry values for the RTP, a stand-in
  `d3d8.dll`, and a table of font replacements (`HKCU\Software\Wine\Fonts\Replacements`) from
  game font names to `Noto Sans CJK SC`, whichever file the system resolves that to.
- Wine finds fonts in the prefix's `drive_c/windows/Fonts` and through fontconfig; the game asks
  GDI for a font by name, in the test game `黑体`.
- `vkxp` takes font directories (`--font-dir`), searched before system fonts, and a
  substitution table for Windows family names.
- On the development machine a static open font is available outside the repository at
  `../ref/fonts/NotoSansCJKsc-Regular.otf` (from a clone: `../../ref/fonts/`).

## Decisions

- Read the family name from the file's `name` table with a small reader in `xtask` (records
  1 and 16, Windows Unicode platform preferred), enough for TrueType and OpenType files; a
  collection (`.ttc`) uses its first face.
- Copy the files into `<out>/fonts/` for vkxp and into the prefix's `Fonts` directory, and
  write the replacements with the family name. Register the fonts in the prefix so that GDI
  enumerates them even when fontconfig does not know them.
- The coordinator verifies the original side on the real game; the flow's sandbox cannot run
  Wine.

## Follow-ups

- Whether vkxp's substitution prefers a font from the given directory for `黑体` over a system
  font is to be checked on the first real comparison.
