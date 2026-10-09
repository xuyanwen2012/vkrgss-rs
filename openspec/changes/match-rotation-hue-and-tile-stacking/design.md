# Design

## Context

- Rules and verification: `local/measurements/README.md`, the sections "CORRECTION 2026-10-09 -
  Sprite angle", "CORRECTION 2026-10-09 - Bitmap#hue_change" and "Tilemap: stacked translucent
  tiles", with the models `sprite-angle-model.py` (cases in `p6cases.py`), `hue-model.py`,
  `tilemap-blend-check.py`, the samples `hue-samples.txt`, the probes, and captured original
  frames of the rotation probe in `local/measurements/angle-frames/` (all in this clone,
  ignored by git). Read the three sections first: they hold the formulas in full, including
  the order of the floating-point products, which matters.
- `vkxp-core`: `Geometry::transform` (`geometry.rs`) builds a 16.16 affine transform for
  rotated sprites that the compositors evaluate per pixel; unrotated sprites are stretch
  quads. `TileCache::compose` (`tilemap.rs`) composes all tiles of one priority with
  `Bitmaps::blt` into a cached bitmap per priority.
- `vkxp-pixel`: `rotate_hue` (`bitmap.rs`) is a rotation that keeps the largest and smallest
  channel, listed as a placeholder.
- The compositors must agree bit for bit and use integer arithmetic; the original's rotation
  accumulates doubles along each row.

## Decisions

- Rotation is computed in the guest, in `f64` exactly as the model, into a cached bitmap of the
  bounding box (transparent where no texel is shown), and extracted as an unscaled quad at the
  box's position with the sprite's opacity, blend mode, tone and colour. The cache is keyed by
  the bitmap's content revision, source rectangle, origin, zoom, angle and mirror; a sprite
  that turns every frame re-renders every frame, which is the cost of exactness. The affine
  quad stays in the protocol but the scene no longer emits it for sprites.
- `sin` and `cos` come from `libm`, which the crate already uses; the README notes that no
  measured case depends on the last bit.
- Hue: the measured integer arithmetic, with provenance `oracle measurement 2026-10-09,
  RGSS104E, hue probe`.
- Tiles: one cached bitmap per priority and map layer instead of one per priority, drawn as
  separate quads in layer order at the same z, so every tile meets the frame through the
  sprite blend. Layers of a priority that hold no tile in view emit nothing.
- Bush depth of a rotated sprite keeps its present source-space split, applied when the cache
  bitmap is rendered (PLACEHOLDER: not measured).

## Risks / Trade-offs

- [More quads per tilemap row] → At most three per priority that is present.
- [Uploading a rotated bitmap every frame for spinning sprites] → Bounded by the sprite's box;
  the upload path already handles bitmaps that change every frame.
