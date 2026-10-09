# Proposal

## Why

Two properties that games set are stored by the scene and not drawn: a sprite's bush depth (the
lower part of a character standing in tall grass is half transparent) and a tilemap's flash
data (tiles that pulse in a colour, used to mark ranges in tactical games). Both are needed for
maps to look as in the original.

## What Changes

- A sprite with a bush depth draws its lowest rows, that many source pixels high, at half
  opacity.
- A tilemap draws a pulsing colour over every cell that has a non-zero entry in its flash data.
- No protocol change: both are expressed with the existing quad and rectangle ops.

Out of scope: comparing the exact pulse curve and blend with the original (marked
PLACEHOLDER).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: bush depth and tilemap flash data are drawn.

## Impact

- Crate `vkxp-core` only.
