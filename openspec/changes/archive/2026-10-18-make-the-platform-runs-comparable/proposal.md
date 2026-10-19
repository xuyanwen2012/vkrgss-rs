# Make the runs on the platforms comparable

## Why

The cross-architecture comparison of 2026-10-18 at `07b24f80` (`vkxp/target/xarch-p2p3/` of
the workspace; scripts `wt/xarch.sh`, `wt/xarch-compare.py` and `target/xarch-p2p3/tools/`)
ran the ten comparison games on macOS (Apple M4) and on the Pixel 7a and Galaxy S24+ against
x86_64. Composition agrees everywhere; four things kept the outputs from being equal:

1. **The oracle's copy of a game keeps file times on macOS and renews them on Linux.**
   `xtask/src/oracle/game.rs` copies with `fs::copy`, which on macOS keeps the source's
   modification time and on Linux gives the copy's time. The tower game loads the newest of
   two saves, so the two platforms loaded different saves (frames 350 and 599 differ). With the
   x86 times the Mac's frames are identical.
2. **The Android app has no `font_own_metrics`.** Its `vkrgss.toml` refuses the key, so XP games
   draw text with other metrics than the desktop run with `--font-own-metrics`; without that
   option on x86 the frames equal the phones'.
3. **Android has no family for Arial.** The substitution list of Arial/Helvetica names Liberation
   Sans, Arimo and DejaVu Sans, none of which Android has; Visions & Voices asks
   `Font.exist?("Arial")` and shows a message when it is false.
4. **One real difference on Android:** in `mh` frame 599, a strip of 31 by 1 pixels at x 231-261,
   y 479 (the last row) is white on both phones and not on x86 or macOS, with or without own
   metrics. Cause unknown.

## What Changes

- The oracle sets the modification time of every file of the copy to that of its source on all
  platforms (and the probe's own files to a fixed time), so a run does not depend on the
  platform's copy.
- The Android `vkrgss.toml` accepts `font_own_metrics` as the command line's
  `--font-own-metrics`.
- The substitution lists of the sans, serif and monospace families end with the families
  Android ships (`Roboto`, `Noto Serif`, `Droid Sans Mono` or what the phones have; check
  `/system/fonts` and `/system/etc/fonts.xml` on both phones), so `Font.exist?("Arial")` is
  true there.
- The cause of the white strip of `mh` frame 599 on Android is found and fixed.
- The cross-architecture runner becomes a command of `xtask` so the comparison can be
  repeated: `cargo xtask xarch run <out>` (this machine or the Mac) and
  `cargo xtask xarch android <serial> <out>`, and `cargo xtask xarch compare <a> <b>`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: file times of the copy.
- `supported-platforms`: equal output across platforms for the comparison games.

## Impact

`xtask` (`oracle/game.rs`, a new `xarch` command), `crates/vkrgss-android` (options),
`crates/vkrgss-text` (substitution lists), whatever the strip's cause needs,
`docs/development.md`, `docs/running.md`, `CHANGELOG.md`. The x86 comparisons with the
original runtime change only where a game reads file times (tower); the original side gets the
same times, so its comparison stays valid.
