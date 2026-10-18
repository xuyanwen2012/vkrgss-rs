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

## Results

### File times (task 1.1)

- `game::prepare` gives every copied file the modification and access times of its source
  (`File::set_times` through a descriptor opened for reading, which the owner of a file may
  use), and the files the oracle writes into the copy itself, the archive with the probe and
  the DLL it places, 2000-01-01 00:00:00 UTC. The unit test
  `the_copy_keeps_the_times_of_the_files_of_the_game` checks two saves with different times
  and the probed archive.
- The vkrgss side of the seven games of `ref/tools/oracle/regress.sh`, run from this branch
  with the same options (`--vkrgss-only`), gives frames identical, pixel for pixel, to those
  of `vkxp/target/regress/` of 2026-10-18 for all seven, tower included. Tower does not
  change there because `regress.sh` runs `tower_exe`, a copy made with `cp -r` whose files
  all have the same time; the copy of the oracle now keeps those equal times, so tower's
  summary stays as it was (frames 100, 350, 599: 3208, 1196 and 1999 differing pixels). The
  tower of `xarch`, which is the game directory itself with its original times, is the one
  whose loaded save changes.

## Out of scope

iOS, release builds, changing what the original runtime does.

## Follow-ups

- The original side of `regress.sh` could not run in the flow that implemented this change:
  its environment denies `mount` to the processes it starts, so `bwrap` stops with "Failed to
  make / slave: Operation not permitted" before Wine starts. The comparisons with the
  original runtime on x86 are to be run by the owner's session (`ref/tools/oracle/regress.sh`
  on this branch); task 1.1 stays unticked until they pass.
