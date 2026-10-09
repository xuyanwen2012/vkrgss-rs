# Proposal

## Why

Measured on the original runtime: a zoomed, unrotated sprite is not sampled at pixel centres
through an affine map. It is a stretch of its source rectangle into an integer screen rectangle
with a fixed-point step, exactly like `stretch_blt`; planes tile such a stretched tile with an
offset in screen pixels; rotated sprites sample at integer pixel offsets from the sprite's
position. vkxp samples at pixel centres through a 16.16 affine transform, so every zoomed
sprite, every zoomed plane and most rotated pixels differ from the original. The same
measurements give exact rules for bush depth with zoom and for three flash cases.

## What Changes

- **BREAKING** (protocol version 3): a quad carries either the affine transform or a stretch
  description (integer destination rectangle and mirror flag), which every backend samples with
  the measured fixed-point rule.
- `vkxp-core` emits stretch quads for unrotated sprites with the measured rectangle
  (`floor(w * zoom)`, `floor(x - ox * zoom)` in 32-bit float), and for planes as repeated
  stretched tiles with the offset in screen pixels.
- Rotated sprites keep the affine form but are evaluated at integer pixel offsets from the
  sprite position, as the measured rule says (not exact at texel boundaries; PLACEHOLDER).
- Bush depth applies to the lowest screen rows; flash: duration 0 shows until the first update,
  a tie between flash and colour alpha goes to the flash, and the decay is the measured integer
  formula.

Out of scope: exact rotation at texel boundaries and at right angles, windows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: quad sampling variants, version 3.
- `frame-composition`: stretch sampling.
- `core-scene`: sprite, plane, bush and flash rules.

## Impact

- Crates `vkxp-proto`, `vkxp-core` (extraction), `vkxp-compose-ref`, `vkxp-vk` (compute
  shader), and every test that builds quads or pins zoomed pixels.
