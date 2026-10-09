# Proposal

## Why

The test game has no title screen: it boots straight into its map. A map is a tilemap, usually
with a panorama and a fog plane, and until those draw there is nothing to look at. The scene
model stores planes and tilemaps but draws nothing for them.

## What Changes

- Planes draw: a bitmap repeated without end over its viewport, with scroll origin, zoom,
  opacity, blend type, colour and tone.
- Tilemaps draw: three layers of 32 by 32 tiles from a tileset and seven autotile sheets,
  scrolled by the tilemap's origin, with autotile patterns, autotile animation, and per-tile
  priorities that place a tile's row in front of or behind sprites.
- Tables can be shared: a table assigned to a tilemap stays connected to it, so later changes
  to the table show up without reassigning it.
- **Temporary deviation from `docs/decisions.md`**: the settled design evaluates a tilemap layer
  as its own op in the compositor. To reach a playable map sooner, this change composes the
  visible tiles on the CPU into cached bitmaps and emits ordinary quads, which needs no protocol
  or compositor change. `docs/decisions.md` is updated to say so; the tilemap op remains the
  target.

Out of scope: the tilemap's flash data (stored, not drawn), bush depth, new protocol ops.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: planes and tilemaps are drawn; the requirement that they draw nothing is
  replaced.

## Impact

- Crate `vkxp-core` only, plus a paragraph in `docs/decisions.md`.
- Bitmaps cached by tilemaps and planes live in the bitmap arena and travel to the host as
  ordinary bitmap deltas; a scroll across a tile boundary re-renders and re-uploads the
  tilemap's cache.
- The Ruby binding's `Plane`, `Tilemap` and `Table` classes build on this.
