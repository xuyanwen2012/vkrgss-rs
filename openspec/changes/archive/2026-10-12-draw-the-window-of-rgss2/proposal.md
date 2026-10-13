## Why

Every scene of a VX game is made of windows, and the first script of one stops at
`Window#openness=`. The window of RGSS2 is not the window of RGSS1: another skin layout, a
stretched background with a tiled pattern, one layer instead of two, an opening animation,
other blink and pause timings, and state that is latched at certain calls. All of it was
measured on the original runtime of VX on 2026-10-12 (`vx-v1-window-REPORT.md` with its
model and cases under `ref/tools/oracle/measurements/`, outside the repository): 490 of 496
captures and both held-out random sets (240 captures, 54 312 960 pixels) are reproduced with
no wrong pixel, on both DLL versions.

## What Changes

- In RGSS2 a window is drawn by the measured rules of RGSS2: skin layout, background,
  frame, openness, contents, cursor, pause sign, arrows, and what is latched when.
- `Window#openness` and `openness=` in RGSS2.
- In RGSS2 a window is one element at its z, without the contents layer at z + 2 of RGSS1.
- XP is drawn exactly as before.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: the window of RGSS2.
- `rgss-scene-classes`: `openness`; the pending list of the method test.

## Impact

- `crates/vkxp-core` (`window.rs` and what it needs of the scene and extraction),
  `crates/vkxp-binding` (`window.rs`, the method test).
- The protocol and the compositors only if the measured rules cannot be said with the ops
  that exist; `design.md` says how they can.
- `docs/compatibility.md`.
- Not touched: bitmap operations, text and fonts (`crates/vkxp-pixel`, `crates/vkxp-text`,
  `crates/vkxp-core/src/bitmap.rs`, `crates/vkxp-binding/src/bitmap.rs` and `font.rs`): the
  change `add-the-bitmap-operations-of-rgss2` runs in parallel there. `graphics.rs` of the
  binding neither: `resize-the-screen-in-rgss2` runs there.
