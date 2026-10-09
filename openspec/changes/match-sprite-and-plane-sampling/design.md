# Design

## Context

- Measurements: `local/measurements/README.md`, sections "stretch_blt sampling", "Sprite zoom
  on screen, angle 0", "Sprite angle", "Flash and bush_depth" and "Plane", with
  `sprite-zoom-samples.txt`, `sprite-rotation-samples.txt`, `flash-bush-samples.txt` and
  `plane-samples.txt` (in this clone, ignored by git; selections are committed as test data).
  Read those sections first.
- `vkxp_proto::Quad` has `clip`, `bitmap`, `src`, `transform` (six 16.16 coefficients, evaluated
  at pixel centres), `opacity`, `blend`, `tone`, `color`. `PROTOCOL_VERSION` is 2.
- `vkxp-core` extracts sprites as affine quads (two for bush depth, split in source space) and
  planes as tiled affine quads, pre-tiling small bitmaps; `vkxp-compose-ref` and
  `crates/vkxp-vk/shaders/compose.comp` evaluate quads and must agree bit for bit.
- A parallel change replaces `vkxp_pixel::stretch_index` with the same fixed-point rule for
  bitmaps; this change needs the rule for quads and defines it where the compositors can share
  it, without depending on that change.

## Decisions

- `Quad` gains `sampling: Sampling` with `Affine(Transform)` and
  `Stretch { dest: Rect, mirror: bool }` in place of the bare transform; version 3.
- Stretch evaluation: `step = (w << 20) / dw` needs 64-bit intermediate products
  (`i * step`); the CPU reference uses `i64`, the shader uses the 32-bit multiply-with-carry
  built-ins or `int64_t` only if the device feature is already required, so that no new device
  requirement appears.
- Core: unrotated sprites and planes become stretch quads as the spec says. A plane is emitted
  as one stretch quad per visible repetition of the tile, clipped by the viewport; the
  pre-tiling of small bitmaps stays as an optimisation only where it leaves pixels unchanged.
- Rotated sprites: the affine transform is built from the measured rule and shifted by half a
  pixel so that the centre-sampling evaluation of the backends lands on the integer-offset
  position (PLACEHOLDER: the original deviates at exact texel boundaries and at 90, 180 and
  270 degrees; the README has a partial explanation).
- Bush depth for unrotated sprites splits the destination rectangle (two stretch quads with
  `i`, `j` counted from the same corner, the second with a clip on the lowest rows); rotated
  sprites keep the source-space split.
- Flash state follows the measured integer decay.

## Risks / Trade-offs

- [Every test that pins zoomed pixels changes] → Recompute with the new rule; the measured
  samples are the authority.

## Follow-ups

- Exact rotation; windows.
