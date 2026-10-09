# Design

## Context

- `vkxp-core`'s scene stores `bush_depth` on sprites and the flash data table on tilemaps and
  draws neither. Sprites are extracted as one quad each; tilemaps as cached bitmaps drawn by
  quads per row and priority.
- The original draws the bush part of a sprite at half opacity, and flashing tiles as an
  additive colour overlay whose strength cycles with `Tilemap#update`.

## Decisions

### Bush depth: two quads

A sprite with an effective bush depth is extracted as two quads with the same transform, clip,
blend, tone and colour: one for the source rows above the bush line and one for the rows below
it with half the opacity (rounded down). Both keep the sprite's place in the draw order. The
split is in source space, so zoom, rotation and mirror need no special case.

### Flash data: overlay quads from a cached bitmap

A per-tilemap overlay bitmap of the visible cell area is rebuilt when the flash table's
revision, the scroll cell or the pulse step changes: each flashing cell is filled with its
colour (each four-bit component times 17) at the pulse's alpha. It is drawn additively as one
quad just above the ground layer and below every priority layer. The pulse is a triangle over
32 updates (PLACEHOLDER: curve, period and peak strength are not yet measured on the original).

## Risks / Trade-offs

- [Rebuilding the overlay every pulse step] → Only when flash data is present and has non-zero
  cells in view; the bitmap is at most the visible area.

## Follow-ups

- The pulse reaches alpha 0 once per period, so a tilemap that is never updated shows no
  flash. Whether the original's pulse has a floor is part of the PLACEHOLDER measurement.
- The flash data is read as repeating beyond its edges, like the map data. Not checked against
  the original.
- The overlay bitmap is uploaded whole on every pulse step while a flashing cell is in view.
  The tilemap op of `docs/decisions.md` is the place to remove that cost.
