## Context

The milestone rule (`docs/decisions.md`, "Order and milestones"): a milestone is reached when
the games of the corpus are identical to the original on compared frames that show that
milestone's outcome, and for the last one also in a save written at the same frame; the
record names the games and their revisions, the DLL of each, the seeds, the input schedules,
the compared frames, the points at which saves were written, and that a repeated run gave
the same. Text: frames may differ inside the rectangles of text in glyph shapes (accepted by
the owner for VX Ace on 2026-10-13); positions, widths and placement are compared.

### The record (measured 2026-10-14, `cargo xtask oracle`, `main` at `51c8d34`)

Machine: the development machine, Wine 11.0 in the harness's sandbox (no GPU device node, no
network), Xvfb, llvmpipe on the vkxp side. Built-in scripts extracted from each game's own DLL
by the user (outside the repository). Seed of the harness: 20261009.

- **Crysalis** (unencrypted, `System/RGSS300.dll`, version 3.0.0.1, `RTP=RPGVXAce`). Schedule:
  the V0 schedule of Crysalis (title, New Game, the introduction confirmed, the map, the
  menu opened, cursor moved, closed, walking into a cave, a battle with one Ratinga fought
  to its end, back on the map). 28 frames: 60, 210, 360 (title and its menu), 1410, 2760,
  4110, 4560 (introduction, messages), 4850 (map), 4900 (inside a transition, not
  comparable), 4950 and 5000 (the menu), 5050 to 5400 (map, walking into the cave), 5450
  (inside a transition), 5500 to 6100 (the battle and its log), 6200 and 6250 (map). 24
  equal pixel for pixel, including every line of text; 4950 and 5000 differ in 3 pixels
  each, the antialiased edge of the glyph "Q" of "Quit" (largest difference 5 of 255),
  which is a glyph shape. The original's pictures equal those of an earlier run of the
  same schedule byte for byte.
- **In Search of Immortality** (`Game.rgss3a`, `System/RGSS301.dll`, version 3.0.1.1,
  `resize_screen` to 640 by 480). Schedule (C every 16 frames until control returns; the
  menu; Save; File 1; Game End; To Title; Continue; File 1): the game is never walked, so no
  random encounter. 15 frames: 60 (logo), 140 and 180 (its account screen and title), 4200
  and 4499 (map), 4700 and 4759 (save screen), 4800 (menu after the save), 4950 (map), 5400
  and 5559 (title with Continue), 5600 and 5659 (load screen), 5760 and 5999 (the map after
  the load): all 15 equal pixel for pixel, text included. Saves written at frame 4760 by the
  game's menu: `Save01.rvdata2` (19018 bytes) and the game's own `AutoSave.rvdata2` (21992
  bytes, written at its map transfers) are byte for byte the original's; the original's
  pictures and both saves equal those of an earlier run byte for byte.

A1 (title screen and menu): Crysalis 60, 210, 360, 4950, 5000; In Search of Immortality 140,
180, 4700, 4759, 4800, 5400, 5559, 5600, 5659. A2 (walking on a map): Crysalis 4850, 5050 to
5400, 6200, 6250; In Search of Immortality 4200, 4499, 4950. A3 (battle, saving and loading):
Crysalis 5500 to 6100; In Search of Immortality the two saves and 5760, 5999 after the load.

Not covered: frames inside a transition (as for VX); the third game of the corpus (Capella's
Promise) was started and captured on the original's side only; F12 and the reset are covered
by tests of vkxp alone; audio is not compared; a save whose music is an Ogg file holds
`Audio.bgm_pos`, which vkxp computes by the measured model and which the original's own runs
can differ in by whole chunks (the save compared here plays an MP3, whose position is 0 on
both sides); the change `match-the-endings-and-loading-of-rgss3` (endings, error reports,
`require`) is not part of 0.3.0.

## Goals / Non-Goals

Goals: the record, the documents, the version. Non-goals: anything of the program beyond
the warning and the usage text.

## Decisions

- The banner: its card of VX Ace said "PLANNED"; the release banner says "SUPPORTED" in the
  same place and colours (a redrawn file is given to the flow; the flow copies it, it does
  not draw).
- The section "Unreleased" of `CHANGELOG.md` becomes the section 0.3.0, with one entry first
  that names support for RPG Maker VX Ace; the entries are kept, merged where two say the same.

## Implementation notes

- The banner file to use: `../../vkxp/target/release-0-3-0/banner.jpg` (outside the clone,
  made by the owner's agent from the owner's picture); copy it to `docs/images/banner.jpg`.
- The version is asserted by two tests: `crates/vkxp/src/args.rs` and the test of `--version`
  in `crates/vkxp/tests/end_to_end.rs`, which names neither the warning nor the usage text.
  Each expects `vkxp 0.3.0` now; the requirement "Version 0.3.0" plainly needs that one-line
  edit outside the listed files, as the release of 0.2.0 made it.

## Follow-ups

- The gate does not pass on 2026-10-14, for a reason outside this change, so task 3.1 stays
  open. Every step passes but `test`, and in `test` every test passes but one:
  `a_busy_main_thread_of_a_copy_gives_way_to_a_new_thread` of `vkxp-ruby192-sys`
  (`tests/fork_busy_main_after_copy.rs`), which this change does not touch. It passed in the
  first of five runs of the gate and failed in the other four, at one assertion or the other
  (the original or the copy reports `false`: its busy main thread kept the lock of the
  interpreter for the whole 30 seconds). Run alone it failed six times in six; pinned to one
  processor (`taskset -c 3`) it passed in 0.02 s. Ruby 1.9.2 hands the lock over by
  `unlock; sched_yield(); lock`, so whether a waiting thread on another processor ever gets
  it depends on the scheduler of the machine. In the first run of the gate the step
  `test` failed instead at `quick_files_pass_under_collector_stress` (`test_thread.rb` ran
  into its time limit), which passed when run alone. The steps `test-unvalidated`, `doc`
  and `openspec` pass when run by hand. Whether the test is changed (its owner crate is
  outside the scope of this change) or the gate is run again on a quieter machine is the
  owner's decision.
