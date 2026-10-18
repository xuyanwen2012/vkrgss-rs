# Design

## Context

- Material (read only): `../../vkxp/target/xarch-p2p3/` — `cmp-*.txt`, `views/` (side-by-side
  pictures, `mh599-galaxy-crop.png`), `nometrics/` (x86 runs without own metrics),
  `mac-rerun/`, `tools/` (`prep.py`, `toml.sh`, `arun.sh`, `cmp.py`: how the phones were run);
  `../../wt/xarch.sh` and `../../wt/xarch-compare.py` (the games, frames and options).
- The owner's games under `/home/doremy/Desktop/rgss/games` and
  `/home/doremy/Desktop/rgss/vkxp/target/playtest-survey` are never committed; the xarch
  command takes their paths as arguments or from a local file under `local/`.
- Phones on USB, free to use for the test APK (do not change system settings; the earlier run
  set `screen_off_timeout`, leave it).

## Decisions

- **File times.** After each copy, `File::set_times` with the source's modified (and accessed)
  time; files the oracle writes itself (probe, Game.ini edits) get a fixed time (the source
  Game.ini's, or the Unix epoch plus a constant), so both platforms see the same order.
  Check that the comparisons of `ref/tools/oracle/regress.sh` still pass on x86; tower's
  numbers may change because the original now loads the other save too — record its new
  summary in `design.md`.
- **font_own_metrics.** A boolean key of the Android options, passed to the host as the
  desktop flag is.
- **Android families.** Read `/system/etc/fonts.xml` on both phones (adb shell cat) and append
  the families both have to the lists, after the Linux ones, so Linux is unchanged.
- **The strip.** Reproduce with `mh` at frame 599 on a phone (`target/xarch-p2p3/tools/arun.sh`
  shows how); compare the guest's composed frame (dumped before presentation) with the
  presented one; check the last row of the readback/dump path on Android and the scaling of the
  overlay (the on-screen buttons are drawn after the game picture: a button edge at the bottom
  row would be exactly this). Fix it at its cause; the dumped frame must be the game's frame.
- **xarch command.** Port `wt/xarch.sh`, `wt/xarch-compare.py` and `tools/arun.sh` into
  `xtask` (Rust), with the game list in a TOML file under `local/` (example committed with
  placeholders, no game names of the owner beyond what the docs already name).

## Out of scope

iOS, release builds, changing what the original runtime does.

## Follow-ups

None yet.
