## Context

- The interpreter starts with the start-up symbol table of the run's runtime; a name that
  is new after that gets the next number of the sequence
  (`third_party/ruby-1.8.7/parse.y`, the symbol patch). Names that vkxp or Ruby 1.8.7 make
  and the table lacks get provisional numbers outside the sequence.
- The measurement: `../../ref/tools/oracle/measurements/vx-v3-save-REPORT.md` (seen from a
  clone under `wt/`), with `vx-v3-save-lazy-probe.rb` and
  `vx-v3-save-lazy-samples.original.txt` (the symbol number after each of 178 steps of a
  small game on the original), `vx-v3-save-first-probe.rb` and its samples (which first
  use interns what), and `vx-v3-save-throwaway.diff` (a patch that was built and run, not
  reviewed; a guide, not the implementation). The report is the authority.

### Measured on the original runtime of VX (2.0.2.1)

1. `owner`, then `observer`, are interned at the first of: `set`, a component writer or
   `Rect#empty` of a `Color`, `Tone` or `Rect`; `Font.new`, `Bitmap.new`, `Sprite.new`,
   `Viewport.new`, `Window.new`, `Plane.new`, `Graphics.snap_to_bitmap`. Not at
   `Color.new`, `Tone.new`, `Rect.new`, readers, `dup`, `_load`, `Table`,
   `Font.default_color=`, `Tilemap.new`, `Graphics.update`, `Input`, `Audio`, `load_data`
   or `save_data`.
2. `__hdll__`, `__dll__`, `__dllname__`, `__proc__`, `__import__`, `__export__` are
   interned, in this order, at the first `Win32API.new` that loads its library.
3. `bmp0` to `bmp8` are interned one by one at the first read or write of the slot of that
   number of `Tilemap#bitmaps`. `Tilemap.new` interns nothing.
4. `recv` is interned at the first `NoMethodError` or `NameError` that method dispatch
   raises (`nil.x`, an undefined local, a private call, `super`, `1 + nil`).
5. The start-up tables of RGSS1 and RGSS2 hold `__member__`; Ruby 1.8.7 names it
   `__members__`, so the first `Struct.new` of a game takes a number the original does not.
6. With all five in place the small game ends at the same number as the original on every
   step, and the save of The Witch's House is byte for byte the original's.

## Goals / Non-Goals

**Goals:** a game of RGSS2 gets the original's symbol numbers on the measured steps, and
the measured save is identical.

**Non-Goals:**
- The provisional numbers: a name of the game that vkxp or Ruby 1.8.7 already made
  (`owner` itself, `text`, `result`, `lines`, `choice`, `command`, `script`, `object`)
  keeps a provisional number, which can order a hash another way than the original. It
  needs a decision about the methods only 1.8.7 has and about the rule of the symbol patch
  and gets a change of its own.
- The lazy names of RGSS1: nothing was measured on the original of XP; RGSS1 behaves as
  before, except for `recv` and `__member__`, which come from the interpreter both share
  and whose names both start-up tables lack or hold alike.
- `require`, `NKF`, and `Win32API.new` with a missing library (found on the way).

## Decisions

- The binding has one place that interns `owner` and `observer` once, called from the
  measured triggers, in RGSS2 only.
- The prelude's `Win32API` of RGSS2 names the six instance variables in `initialize` after
  the library was found, in the measured order. If vkxp's emulation has no use for one of
  them it is still interned there, with a comment that says why.
- The hidden references of a tilemap of RGSS2: the slots are named `bmp0` to `bmp8` and
  interned at the first access of each slot; the back reference of the bitmap array uses
  a name of the start-up table or no name at all, so `Tilemap.new` interns nothing.
- `error.c` interns `recv` where a name error of method dispatch is made, and `struct.c`
  uses `__member__`. Both are recorded in the patch log of the vendored interpreter with
  the measurement as their reason, and the patch chain is regenerated as the log asks.
- The test replays `vx-v3-save-lazy-probe.rb`'s steps in a game of RGSS2 through the
  binding and compares the number after each step with the committed numbers of the
  original (names of the probe's own and numbers only). Steps whose trigger is a non-goal
  are listed in the test with the reason.

## Implementation notes

- The prelude is one file for RGSS1 and RGSS2 and cannot tell them apart, so
  `Win32API#initialize` calls a host function, `vkxp_win32api_names`, which interns the six
  names in a game of RGSS2 and does nothing in RGSS1. vkxp loads no library, so every
  `Win32API.new` counts as one that found its library (the missing library is a non-goal).
- The back reference of the bitmap array is kept under `bitmaps`, a name of the start-up
  table of RGSS2.
- The test of the measured steps compares the numbers themselves, not differences: the game
  of the test starts at the original's number (2081) and ends at 2273, and no step is left
  out. Its built-in scripts are a stand-in of its own with `RPG::Map`, `RPG::BGM` and
  `RPG::BGS`, because the step "RPG marshal" interns the three class paths on the original
  and the built-in scripts of the original are not in the repository. What a step ends with
  (`ok` or an exception class) is not compared.

