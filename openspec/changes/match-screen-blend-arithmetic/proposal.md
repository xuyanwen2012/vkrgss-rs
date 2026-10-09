# Proposal

## Why

The oracle harness now shows the test game following the same course as in the original, and
most remaining differences are single units in translucent pixels: vkxp blends with textbook
formulas. A probe that draws every combination of source value, pixel alpha and opacity in the
original runtime has been measured, and integer formulas have been found that reproduce every
measured value exactly for sprites drawn to the screen: normal, additive and subtractive
blending, the combination of pixel alpha and opacity, the sprite colour and the tone offsets.

## What Changes

- The pixel arithmetic for compositing onto the opaque frame is replaced by the measured
  formulas, in `vkxp-pixel` and in the GLSL that mirrors it, and marked as reverse-engineered
  in the provenance list.
- Blending into a bitmap whose destination has an alpha below 255 (`Bitmap#blt`) keeps its
  placeholder formula; it has not been fitted yet.
- Tests assert samples measured on the original.

Out of scope: `blt` into translucent bitmaps, the grey part of the tone, sampling positions of
zoomed and rotated sprites, viewport effects.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `frame-composition`: exact blend arithmetic on an opaque destination.

## Impact

- Crates `vkxp-pixel`, `vkxp-compose-ref` and `vkxp-vk` (shader source and its tests), and
  tests elsewhere whose expected pixels were computed with the old formulas.
