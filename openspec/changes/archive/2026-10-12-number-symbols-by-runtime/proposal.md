## Why

A symbol's number decides the order of instance variables and so the bytes of a save file.
The interpreter numbers the names that exist at start-up from a table measured on the
original runtime of XP, and continues the sequence where that runtime does. The original
runtime of VX has another table: 1996 names instead of 2002, 352 of them not in the table of
XP, and its first new name gets 16089 where XP gives 16193 (measured on 2026-10-12 on
`RGSS202E.dll` 2.0.2.1 and on the 2.0.0.1 file of a game; both give the same). With the table
of XP a VX game would write saves that differ from the original's, which milestone V3
compares. `docs/decisions.md` says that the runtime is chosen before the interpreter is
initialised, because the table is compiled in.

## What Changes

- The measured table of RGSS2 is committed beside that of RGSS1
  (`crates/vkxp-ruby-sys/data/symbols.startup.rgss2.txt`, added with this proposal).
- Both tables are compiled in. An option of the interpreter's start names one; it is applied
  before Ruby is initialised and cannot change afterwards. The default is the table of
  RGSS1, so every existing caller behaves as before.
- The recorded patch of the symbol numbers is changed accordingly and `PATCHES.md` says so.
- The design records which recorded patches are shared by XP and VX and which follow the
  runtime, as the decisions ask of the first change that implements VX.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the requirement "Symbol numbers as in the original runtime" is stated by
  runtime and gets the scenarios of RGSS2.

## Impact

- `crates/vkxp-ruby-sys` (`build.rs`, `src/interp.rs`, `src/ffi.rs`, tests, `data/`),
  `third_party/ruby-1.8.7` (`parse.y`, `parse.c`, the two patches of the symbol numbers,
  `PATCHES.md`, `README.vkxp.md`).
- Not touched: `crates/vkxp`, `crates/vkxp-binding`. The program keeps starting the
  interpreter with the default options; passing the table of the detected runtime is a
  later change, once this one and `detect-the-runtime-and-open-vx-games` are merged.
