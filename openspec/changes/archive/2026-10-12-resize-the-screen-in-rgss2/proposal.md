## Why

`Graphics.resize_screen` is the last function of `Graphics` that RGSS2 has and vkxp lacks.
Games of VX call it to get a frame of 640 by 480. Its limits were measured on the original
runtime of VX on 2026-10-12 (`vx-v1-api-REPORT.md` section 3.2 under
`ref/tools/oracle/measurements/`, outside the repository).

## What Changes

- In RGSS2 `Graphics.resize_screen(width, height)` sets the size of the frame, each side
  limited as measured. Frames, `Graphics.width`, `Graphics.height` and
  `Graphics.snap_to_bitmap` follow it.
- XP is unchanged; the function does not exist there.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-modules`: `Graphics.resize_screen`.

## Impact

- `crates/vkxp-binding` (`graphics.rs`, tests), `crates/vkxp-core` (`scene.rs`: the size of
  the scene can change).
- `crates/vkxp-host` and `crates/vkxp-vk` only if a frame of another size than the one
  before is not shown correctly today.
- Not touched: the window, bitmaps and text: the changes `draw-the-window-of-rgss2` and
  `add-the-bitmap-operations-of-rgss2` run in parallel in `crates/vkxp-core`
  (`window.rs`, `bitmap.rs`, `text.rs`) and in the binding's `window.rs`, `bitmap.rs`,
  `font.rs`.
