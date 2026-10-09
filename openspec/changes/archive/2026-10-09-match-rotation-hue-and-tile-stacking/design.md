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

## Follow-ups

- The cache key of a rotated sprite also holds where the bounding box and its rendered part
  lie against the sprite's position, and the bush depth. The position enters the measured
  rule only through these whole-number offsets, so a sprite that moves keeps its bitmap, and
  the pixels stay exact where the rounding of a corner depends on the position.
- Only the part of the bounding box inside the sprite's viewport or the screen is rendered,
  so the cached bitmap is never larger than the screen. A row is still walked from the left
  edge of the box, as the rule accumulates from there; a box of more than `MAX_BITMAP_PIXELS`
  cells is not drawn. Whether the original runtime draws anything at such zooms is not
  measured.
- `Scene::dispose_sprite` takes no bitmap arena, so the cached bitmap of a disposed rotated
  sprite is disposed by the next `Scene::extract`.
- Bush depth of a rotated sprite: the texels of the lowest source rows get half their alpha in
  the cached bitmap. Before, they were a second quad with half the opacity; the two differ by
  rounding. Still a PLACEHOLDER, to be replaced by a measurement.
- A negative zoom of a rotated sprite goes through the measured rule as it is written; what
  the original runtime draws there is not measured.
- Right angles of very small sprites draw little or nothing by the measured rule (a 2 by 1
  bitmap at 90 degrees shows no pixel). This follows from the rule, which reproduces the
  measured 200 by 150 and 329 by 329 frames; no frame of a tiny sprite was captured.
- `Geometry::transform` and the affine quad remain for windows and tilemaps, which use them
  unrotated.
- The committed hue samples were measured on RGSS103J; the arithmetic was read from RGSS104E.
  All of them are reproduced. The 634 texels of the game sprite are not committed (game art).
