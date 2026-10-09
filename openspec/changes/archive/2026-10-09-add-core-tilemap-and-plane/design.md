# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-core` has the scene arena of `add-core-scene`: sprites, viewports and windows are
  extracted into quads and rectangle effects, ordered by `(z, stamp)` inside each viewport.
  Planes and tilemaps exist as property holders. Windows already keep cached bitmaps in the
  bitmap arena and rebuild them when their inputs change.
- The protocol's `Op::Plane` and `Op::TilemapLayer` are reserved names that both compositors
  reject. Adding real ops means a protocol version and work in both compositors.
- `docs/decisions.md` says a tilemap layer is an op type evaluated in the shader.
- The original runtime's autotile table was read from its DLL (48 patterns, four quarters each,
  as column and row of a 6 by 8 grid of 16-pixel quarters) and equals the table published with
  community tilemap scripts. With quarters numbered 1 to 48 row by row it is:

  ```text
  27 28 33 34 |  5 28 33 34 | 27  6 33 34 |  5  6 33 34 | 27 28 33 12 |  5 28 33 12 | 27  6 33 12 |  5  6 33 12
  27 28 11 34 |  5 28 11 34 | 27  6 11 34 |  5  6 11 34 | 27 28 11 12 |  5 28 11 12 | 27  6 11 12 |  5  6 11 12
  25 26 31 32 | 25  6 31 32 | 25 26 31 12 | 25  6 31 12 | 15 16 21 22 | 15 16 21 12 | 15 16 11 22 | 15 16 11 12
  29 30 35 36 | 29 30 11 36 |  5 30 35 36 |  5 30 11 36 | 39 40 45 46 |  5 40 45 46 | 39  6 45 46 |  5  6 45 46
  25 30 31 36 | 15 16 45 46 | 13 14 19 20 | 13 14 19 12 | 17 18 23 24 | 17 18 11 24 | 41 42 47 48 |  5 42 47 48
  37 38 43 44 | 37  6 43 44 | 13 18 19 24 | 13 14 43 44 | 37 42 43 48 | 17 18 47 48 | 13 18 43 48 |  1  2  7  8
  ```

  Pattern numbers run left to right, top to bottom, 0 to 47; each group is top-left, top-right,
  bottom-left, bottom-right.
- The default game scripts give a character sprite the z of its feet's screen y plus 32, plus
  31 if it is taller than a tile, and assign map tables once: `tilemap.map_data = $game_map.data`.
  Scripts may then change that table and expect the map to follow.

## Goals / Non-Goals

**Goals:**

- A scrolling map and its panorama and fog look right and rank correctly against character
  sprites, with no protocol or compositor change.
- Scrolling within a tile costs nothing but a moved quad.

**Non-Goals:**

- The tilemap op of `docs/decisions.md`. This change is written so that op can replace the
  cache later without touching the binding.
- Drawing flash data. Bush depth.
- Bit-exact agreement with the original's tile compositing.

## Decisions

### Tilemaps are composed into cached bitmaps

For each visible tilemap the scene keeps, in the bitmap arena, up to six bitmaps covering the
visible cells: the area's width and height rounded up to whole tiles, plus one tile in each
direction, so the cache stays valid while the origin moves within a tile.

- `ground`: every tile of priority 0, layers 0, 1, 2 drawn in that order with `blt`;
- `priority[p]` for p from 1 to 5: the tiles of that priority, same layer order, created only
  when a visible tile has that priority.

Extraction emits one quad for `ground` at z 0, and for each priority bitmap one quad per row of
cells that contains a tile of that priority, with the source rectangle of that row and the z of
`specs/core-scene` (row top y plus `(p + 1) * 32`). All of them carry the tilemap's stamp and the
viewport's clip, and sit at `-(ox mod 32)`, `-(oy mod 32)` relative to the cache's first cell.

The cache is rebuilt when any of these change: the first visible cell (a scroll across a tile
boundary), the size of the visible area, the revision of the map data or priorities table, the
identity or content of the tileset or an autotile bitmap (the arena's change tracking), or the
animation frame of an autotile that is visible. A rebuild redraws everything; with at most
21 by 16 cells in three layers that is a few thousand small transfers.

An autotile tile is four 16 by 16 transfers from the current frame of its sheet, at the
positions the table gives; a 32-pixel-high sheet is one 32 by 32 transfer.

Alternative considered: one quad per tile. Rejected: about a thousand ops per frame for the
ground alone, re-sent every frame.

Alternative considered: the shader-evaluated tilemap op now. Rejected for this step only: it
needs table resources in the protocol and matching work in both compositors before anything
appears on screen. `docs/decisions.md` gets a paragraph recording that the CPU cache is the
current implementation and the op is still the plan.

### Shared tables

`SharedTable` is a reference-counted handle to a `Table` with a revision counter that every
write through the handle increments. A tilemap stores handles for map data, priorities and
flash data. The binding gives every Ruby `Table` object such a handle, so assigning a table to a
tilemap shares it. `Table` itself stays the plain value it is; the handle is single-threaded
(the scene lives on the guest thread).

### Planes are tiled quads

With the bitmap size `(bw, bh)`, zoom `(zx, zy)` and the clip area's top-left corner as origin,
the plane emits one quad per repeat that overlaps the area. All repeats share the linear part
of the transform (`1/zx`, `1/zy`) and differ only in a translation that is a whole multiple of
`bw` or `bh` in source pixels, so every screen pixel maps into exactly one repeat. Opacity,
blend mode, tone and colour are set on each quad.

When the area would need more than 64 repeats (a small pattern), the plane first builds a
cached bitmap that repeats the source enough times to bring the count under that limit, rebuilt
when the source changes, and tiles that instead.

### Tile ranks and ties

Tilemap pieces enter the viewport's ordering like any element: `(z, stamp)` with the tilemap's
stamp. A sprite created after the tilemap therefore wins a tie in z, which is what the default
scripts rely on for characters standing exactly one row below a priority tile.

## Risks / Trade-offs

- [Rebuilding and re-uploading up to six screen-sized bitmaps on every tile-boundary crossing] →
  At walking speed that is every eighth frame; measured in the tasks. The tilemap op removes it
  later.
- [Animated autotiles force a rebuild every 16 frames] → Same cost as a boundary crossing, only
  while an animated autotile is visible.
- [Wrap-around is assumed, not verified] → Marked `PLACEHOLDER`; maps as large as the screen or
  larger never show it under the default scripts.
- [A reference-counted table in a crate that is otherwise value-oriented] → Confined to one
  type; needed for the semantics scripts rely on.

Behaviour to confirm against the original runtime later: wrap-around; the z of priority tiles
and the tie rule; autotile animation speed; what identifiers 1 to 47 draw; the treatment of
undersized autotile sheets; plane sampling under zoom.

## Follow-ups

- A plane zoomed out so far that its pre-tiled bitmap would need more than 2^22 pixels is not
  drawn (for a small pattern over a 640 by 480 area: a zoom below about 0.04). Staying under
  64 quads there needs either a larger cached bitmap or another way of drawing; neither is
  named above, so the case is left undrawn. Is it worth more?
- Whether the origin of a viewport scrolls its planes and tilemaps is not stated above. They
  follow it here, as sprites do, marked `PLACEHOLDER`.
- An invisible plane or tilemap gives its cached bitmaps back and composes them again when it
  is shown; one that is hidden by its viewport keeps them. If scripts toggle visibility every
  few frames, keeping them may be the better trade.
- One rebuild of a full 21 by 16 cell cache takes about 7.5 ms in the release profile on the
  development machine, which a walking player pays every eighth frame. The tilemap op removes
  it; until then, composing only the column or row that scrolled in would cut it.
