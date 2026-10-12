## Why

A game of RPG Maker VX is now recognised and started, but the guest it gets is the guest of
XP: the interpreter numbers symbols by the table of RGSS1, the frame is 640 by 480 and the
frame rate starts at 40. The original runtime of VX starts with its own symbol table, a frame
of 544 by 416 and 60 frames per second (measured, `vx-v0-REPORT.md` and `vx-v1-REPORT.md`
under `ref/tools/oracle/measurements/`, outside the repository). Everything that follows for
milestone V1 stands on these three.

## What Changes

- The program starts the interpreter with the symbol table of the run's runtime (the table
  of RGSS2 exists since `number-symbols-by-runtime`; nothing selects it yet).
- The scene of a game of RGSS2 is 544 by 416 and its frame rate starts at 60. The window and
  a headless dump follow the size.
- `Graphics.width` and `Graphics.height` exist in RGSS2 and give the size of the frame.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the symbol numbers a game's scripts see follow the runtime.
- `rgss-modules`: the starting frame rate by runtime; `Graphics.width` and `Graphics.height`.
- `game-launch`: the size of the frame of a run.

## Impact

- `crates/vkxp` (the two places that start the interpreter; the size given to the host),
  `crates/vkxp-binding` (`guest.rs`, `runtime.rs`, `graphics.rs`, the testing host).
- `docs/running.md` where it says how far a game of VX gets.
- Not touched: the protocol, the host, the compositors and `crates/vkxp-core` beyond a
  constructor of the scene if one is missing: the change
  `show-brightness-and-rgss2-transition-steps` runs in parallel on those.
