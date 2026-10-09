# Proposal

## Why

Measurements on the original runtime now give exact integer rules for `Bitmap#blt` into a
bitmap with any alpha, for `stretch_blt` sampling, and for the grey part of the tone. vkxp
uses textbook formulas there: `blt` with alpha differs from the original in about 93% of the
measured cells, stretching differs for clipped and mirrored rectangles and some ratios, and
the grey tone is off by up to 2. Windows and tilemaps are built with these operations, so the
differences show on screen.

## What Changes

- `blt` and `stretch_blt` blend with the measured formula, including its five measured
  exceptions; `fill_rect` and `set_pixel` keep replacing.
- `stretch_blt` picks source texels by the measured fixed-point step, with the measured rules
  for source rectangles that leave the source bitmap and for negative widths (mirroring), in
  `blt` as well.
- Colour components written into a bitmap are truncated and taken modulo 256 instead of being
  clamped below zero, as measured; `Color` and `Tone` clamp only their upper bound on
  construction.
- The tone's grey part uses the measured luminance and blend, in the CPU reference and in the
  shader.

Out of scope: `hue_change` (no exact formula yet), sampling of sprites on screen.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-bitmap`: exact `blt`, `stretch_blt` and colour storage.
- `frame-composition`: exact grey tone.

## Impact

- Crates `vkxp-pixel`, `vkxp-core` (bitmap operations and value types), `vkxp-binding` only
  where a test pins old values, `vkxp-vk` (`shaders/pixel.glsl` for the tone) and
  `vkxp-compose-ref`.
