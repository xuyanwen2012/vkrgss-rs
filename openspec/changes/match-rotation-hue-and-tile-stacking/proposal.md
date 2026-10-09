# Proposal

## Why

A diagnosis of every remaining non-text difference in seven games against the original found,
besides JPEG decoding (a separate change), three places where vkxp departs from the original,
each now with an exact rule that reproduces every measured pixel:

- Rotated sprites. The original computes a bounding box from the rotated corners, walks it row
  by row accumulating the source position in double precision, truncates, and leaves out the
  box's last row and column; the result goes into a temporary bitmap that is then drawn like an
  unrotated sprite. vkxp evaluates a fixed-point transform per pixel: up to all pixels of a
  rotated sprite differ, and the coverage differs (29651 against 30000 pixels in one case).
- `Bitmap#hue_change`. The original converts to an integer HSL form and back; vkxp rotates the
  hue keeping the largest and smallest channel. All 634 opaque texels of a measured sprite
  differ, by at most 2.
- Tilemaps. The original blends every tile on the screen by itself, in ascending priority and
  then layer order; vkxp first composes the tiles of one priority into a bitmap. The two differ
  whenever two translucent tiles of one priority are stacked without an opaque tile of that
  priority below.

## What Changes

- Sprites with an angle other than exactly 0 are rendered by the measured rule into a cached
  bitmap and drawn unscaled from it.
- `hue_change` follows the measured integer arithmetic.
- Tilemap layers are kept apart so that each tile is blended on the screen by itself.

Out of scope: rotation inside a viewport with an origin and bush depth of rotated sprites
(not measured), the order of tiles of different rows against each other.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: rotated sprites and tile stacking.
- `core-bitmap`: hue change.

## Impact

- Crates `vkxp-core` (sprite extraction, tilemap cache), `vkxp-pixel` (hue), and tests that pin
  rotated pixels or hue values.
