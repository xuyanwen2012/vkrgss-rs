## Context

`docs/decisions.md` says: "Version 0.2.0 is released when V3 is reached; it does not wait for
VX Ace", and among the risks: "At 60 frames per second a frame has 16.7 ms, and waves, blur
and snapshots are new costs. Frame times of representative VX scenes are measured before
0.2.0." The specification `supported-runtimes` says a runtime is supported when its
milestones are done. V1 to V3 are done since 2026-10-12.

### Measured frame times (2026-10-13)

Release build of `main` at `0bf50cf`, `--headless --no-audio --frame-stats`, which neither
waits between frames nor presents; AMD Ryzen 5 9600X, its integrated GPU with RADV (Mesa
26.2.4), frames of 544x416. The rows are those `docs/running.md` describes under "Frame
times". The logs and the input schedules are kept outside the repository.

| Run | Frames | `guest` mean | `guest` largest | `submit` mean | `submit` largest | `compose (device)` mean | `compose (device)` largest |
|---|---|---|---|---|---|---|---|
| Visions & Voices: title, name input, maps with messages and weather, a battle, walking, the menu over its blurred snapshot | 13500 | 0.208 ms | 39.611 ms | 0.742 ms | 2.937 ms | 0.689 ms | 1.890 ms |
| The Witch's House: title, opening, walking on its first map | 3200 | 0.394 ms | 32.654 ms | 0.825 ms | 2.814 ms | 0.751 ms | 1.738 ms |

`encode`, `bin` and `upload` together have a mean below 0.01 ms in both runs and a largest
single value of 1.215 ms (`upload`).

The guest's own rows, which are written per 600 frames:

- `guest scripts`: the highest mean of a block is 0.543 ms (Visions & Voices, the battle)
  and 0.409 ms (The Witch's House, walking). The largest single frames are 37.8 ms, 28.6 ms
  and 26.4 ms in Visions & Voices and 28.4 ms in The Witch's House, each in a block in which
  a scene is entered (the name input, the menu and the return from it, the start of the
  game). The statistics give the largest value of a block and not the frame it belongs to,
  so that these are the frames of the scene changes is an inference from the blocks.
- `guest bitmap operations`: largest 11.6 ms in the block in which the battle begins, and
  2.2 ms in the blocks in which the menu takes and blurs its snapshot; the mean of every
  block is below 0.01 ms.
- `guest text`: largest 8.0 ms; the mean of every block is below 0.02 ms.
- `guest extraction`: largest 1.9 ms; the mean of every block is below 0.02 ms.

Reading: a frame costs about 1 ms in the mean on this machine, a sixteenth of the 16.7 ms it
has. Single frames in which a scene is built take more than one frame's time, up to 40 ms,
on the game's side. How long the original takes for the same frames was not measured.
Nothing was measured on a slower processor or with a window.

## Goals / Non-Goals

Goals: the program and the documents say what the milestone table says; the frame times are
on record; the version is 0.2.0.

Non-goals: making the frames of scene changes shorter; anything about VX Ace beyond leaving
its state "planned"; tagging or pushing, which the owner's side does after the merge.

## Decisions

- The requirement "Experimental runtime" is removed with its code and its tests, not kept
  for VX Ace. A game of VX Ace still ends at once with the message it has today; what a
  half-working VX Ace says is decided with the changes of VX Ace.
- The section "Before VX is done" of `docs/decisions.md` stays as the record it is; a
  sentence after it says that V3 was reached on 2026-10-12 and that the rule has ended.
- The frame times go into `docs/decisions.md` under the risk that asked for them, as the
  table and the reading above, shortened where it helps. No performance requirement is
  made of them.
- `CHANGELOG.md`: the section 0.2.0 is written from the archived changes since `v0.1.0`
  (`git log v0.1.0..HEAD`), grouped as the 0.1.0 section is (Added, Changed, Fixed), at the
  level of what a user notices; it does not list changes one by one.

## Implementation notes

- `Cargo.lock` follows the version (`cargo metadata` or a build updates it; the release
  workflow builds with `--locked`).
- The README's "Status" section names 0.2.0 and what it adds: RPG Maker VX. Its sentences
  on exactness name the games of VX as the milestone table does, without claiming more.
