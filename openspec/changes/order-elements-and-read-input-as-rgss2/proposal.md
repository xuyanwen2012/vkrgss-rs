## Why

The measurement of sprites, viewports, planes, mask transitions and input on the original
runtime of VX (2026-10-12, `vx-v1-sprite-REPORT.md` under `ref/tools/oracle/measurements/`,
outside the repository; 73 548 800 pixels and 12 084 input updates with none wrong, both DLL
versions) found rules that differ from RGSS1 outside the sprite's own drawing: the order of
elements of equal z, the flash of a viewport, the tile size of a plane, the repeat timing
and the keys of `Input`, and the default vagueness of a masked transition. The menu of a VX
game depends on the first and on the input timing.

## What Changes

- In RGSS2 elements are drawn in ascending order of (z, y, creation), y being that of a
  sprite and 0 for everything else.
- In RGSS2 a viewport's flash colour is applied after its colour instead of replacing it,
  and `flash(nil, duration)` leaves the viewport undrawn and without tone and colour.
- In RGSS2 the tile of a plane is sized in double precision.
- In RGSS2 `Input` repeats after 24 updates and then every 6, and the keyboard maps to the
  buttons as in VX.
- In RGSS2 the vagueness of a masked transition defaults to 40; the thresholds of a masked
  transition of RGSS2 are confirmed as measured.
- In RGSS2 `blend_type=` raises for a value other than 0, 1, 2.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: order, viewport flash, plane tile size by runtime.
- `core-input`: repeat timing by runtime.
- `host-session`: the keys of VX; the mask thresholds of RGSS2 as measured.
- `rgss-modules`: the default vagueness; `blend_type=`.

## Impact

- `crates/vkxp-core` (`scene.rs`, extraction order, `plane.rs`, viewport effects,
  `input.rs`), `crates/vkxp-binding` (`graphics.rs`, `input.rs`, the `blend_type` writers),
  `crates/vkxp-host` (key map by runtime, the mark on the mask threshold), `crates/vkxp`
  (the runtime given to the host), `docs/running.md` (the keys of VX).
- Not touched: the drawing of a sprite itself (`draw-the-sprite-of-rgss2`).
