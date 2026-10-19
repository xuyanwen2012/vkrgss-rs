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

### Android options and fonts (task 2.1)

- `/system/fonts` of both phones (Pixel 7a on Android 17, Galaxy S24+ on Android 16) holds
  `Roboto` (`Roboto-Regular.ttf`, a variable font whose file differs between the two),
  `Noto Serif` and `Droid Sans Mono` (the same files on both), none of the Linux families of
  the lists. They end the lists of the sans, serif and monospace families, so Linux and
  macOS resolve as before.
- A generated game of RGSS2 and one of RGSS3 that write `Font.exist?("Arial")` into a file
  wrote `true` on both phones, with the APK of this branch.

### The strip (task 3.1)

- Cause: a font, not the GPU, the readback or the overlay. The dump of a window is read from
  an off-screen target that the overlay is never drawn into. In `mh` frame 599 the end
  credits begin to scroll in from the bottom: the script draws them into a bitmap with
  `font.name = ["Courier New", "黑体"]`, bold, size 24, and copies one row of it after the
  other into the last rows of the screen with `stretch_blt`, widened, which gives the soft
  ends of the strip. On x86 `Courier New` is replaced by Liberation Mono and on macOS it is
  installed, and the top row of the first line of credits is empty there. On Android the
  list of `Courier New` named no installed family, so the request fell to `黑体`, that is
  Noto Sans CJK SC, whose glyphs reach the top row of the line; that row is the strip.
- Fix: the monospace list ends with `Droid Sans Mono` (task 2.1). With the APK of
  `vkxp/target/apk/` of 2026-10-18 11:08, built before it, the Galaxy S24+ still draws the
  strip in frame 599 (31 pixels at x 231-261, y 479); with the APK of this branch frames 570
  to 599 of `mh` are identical to those of x86 on both phones.

### xarch command (task 4.1)

- `cargo xtask xarch run|android|compare` (`xtask/src/xarch.rs`) ports `wt/xarch.sh`,
  `wt/xarch-compare.py` and `tools/arun.sh`. `run` calls the oracle in the process with the
  arguments the script gave it; `android` makes the oracle's copy with the probe of the
  vkrgss side itself (`tools/prep.py` rebuilt it from the x86 output), writes the
  `vkrgss.toml` that `tools/toml.sh` wrote, now with `font_own_metrics`, and runs it as
  `arun.sh` did, with the steps of `android-run` that `apk.rs` now shares; `compare`
  compares as `xarch-compare.py` and prints the counts of `tools/cmp.py`. The list is
  `local/xarch.toml`; `xtask/xarch.example.toml` shows its keys with placeholders. Both kinds
  of run are given the built-in scripts of the list, so that XP and VX games read the same
  file on the desktop and on the phone.
- Run on 2026-10-18 with the ten games of `wt/xarch.sh`: `xarch run` on this machine
  (llvmpipe) and `xarch android` on the Pixel 7a and the Galaxy S24+; `xarch compare` of
  this machine with each phone gives "all identical": every frame (84) and every save (17)
  of the ten games.
- Two more causes showed on the phones once the strip was gone, both fixed:
  - Visions & Voices raised `NoMethodError` in a battle from frame 6775 on. Its
    `Game_Battler` asks `attacker.id <= 8` of an enemy, which has no `id` and answers with
    `Object#id`, the address of the object; bionic tags heap pointers in their top byte
    (`0xb4…` in the inspected object), so the id was negative on Android and positive
    elsewhere. The guest process now calls `mallopt(M_BIONIC_SET_HEAP_TAGGING_LEVEL,
    M_HEAP_TAGGING_LEVEL_NONE)` before its interpreter starts (`vkrgss-guest/src/heap.rs`,
    called from `run_guest` of `vkrgss-guest-main`). These two crates are outside the scope
    the run was given; the edit is the few lines the requirement "All platforms" needs.
  - A run of the ten games takes longer than the screen timeout of the phones (30 minutes);
    when the screen went off the game waited in the background until the timeout of the
    run. The game activity now sets `KEEP_SCREEN_ON` while a game runs, instead of a change
    to the settings of the phones.
- The Mac is not reachable from this machine: the run of `xarch run` there and its
  comparison with this machine are done at the merge by the owner's session.

## Out of scope

iOS, release builds, changing what the original runtime does.

## Follow-ups

- The original side of `regress.sh` could not run in the flow that implemented this change:
  its environment denies `mount` to the processes it starts, so `bwrap` stops with "Failed to
  make / slave: Operation not permitted" before Wine starts. The comparisons with the
  original runtime on x86 are to be run by the owner's session (`ref/tools/oracle/regress.sh`
  on this branch); task 1.1 stays unticked until they pass.

## Checked at the merge (owner's session, 2026-10-18)

- `ref/tools/oracle/regress.sh` on x86 at `226b6fda` (this change merged): all seven XP games
  give the same summaries as the baseline in `wt/base` (tower included), and the two VX Ace
  games equal the runs of 2026-10-14, their saves byte for byte equal to the original's.
