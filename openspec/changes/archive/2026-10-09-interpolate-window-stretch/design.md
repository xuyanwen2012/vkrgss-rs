# Design

## Context

- `crates/vkxp-core/src/window.rs`: `draw_background` and the centre stretch in `draw_cursor`
  call the bitmap arena's `stretch_blt` with `stretched_part(..)` giving 127 or 27, which is a
  nearest stretch.
- The corrected rule and its verification are in `local/measurements/README.md` under
  "CORRECTION 2026-10-09 - Window: the "numerator 127" stretch is bilinear", with a reference
  function `bilerp` in `local/measurements/window-diag/model2.py` and probe captures next to
  it (all ignored by git). Read the README section first: it states the handling of alpha, of
  rows without colour, and which stretches stay nearest (small backgrounds, 1:1 copies, cursor
  edge strips, the tiled background).
- Fractional precision: 26 to 32 bits could not be told apart by the measurement; 32 is used.
- `vkxp_core::Window` has `set_active` and `set_pause`, which keep the counters as measured;
  `vkxp-binding` assigns the fields directly.

## Decisions

- A window-private bilinear stretch in `vkxp-core`, used only for the two reduced-numerator
  cases; `Bitmap#stretch_blt` is untouched (not measured to interpolate anywhere).
- Committed samples come from the probe captures of the corrected measurement (non-ramp
  skins); the earlier ramp samples stay valid for both rules.
- The binding calls `set_active` and `set_pause`.

## Follow-ups

- A skin smaller than the layout: the bilinear stretch counts texels outside the skin as
  transparent. The original was not measured with such a skin on this path (the nearest
  stretch has its measured clipping rules in `Bitmap#stretch_blt`).
