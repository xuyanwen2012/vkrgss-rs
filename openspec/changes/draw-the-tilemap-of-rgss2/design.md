## Context

- The tilemap of RGSS1: `crates/vkxp-core/src/tilemap.rs` extracts `Op::TilemapLayer` ops,
  which both compositors evaluate from the map table and the bitmaps (requirements
  "Tilemap layer evaluation" and following of `frame-composition`). `docs/decisions.md`
  ("Several runtimes"): the tile system of VX is an op kind that the CPU reference and the
  compute shader both evaluate from the start; no cache of composed maps comes first.
- The measurement: `../../ref/tools/oracle/measurements/vx-v2-tilemap-REPORT.md` (seen from
  a clone under `wt/`), its model `vx_v2_tilemap_model.py` (the tables `FLOOR`, `WALL`,
  `FALL`), `vx_v2_tilemap_topics.py` and `vx_v2_tilemap_sheets.py` (the cases and the
  synthetic sheets). The report is the authority for every rule below; where this text is
  shorter than the report, the report decides. Class behaviour: `vx-v1-api-REPORT.md`
  section 3.8.

### Measured rules (RGSS2, both DLL versions identical; C truncating division)

0. A tilemap is two planes of 672 by 512 pixels (21 by 16 cells) among the elements of its
   viewport: the lower at z 0 and the upper at z 200, each ordered as an element with y 0.
   Both are drawn anew every frame. `X = ox + viewport.ox`, `Y = oy + viewport.oy`; the
   first cell is `floor(X / 32) mod xsize` (likewise y); the corner of a plane is the
   corner of the viewport's rectangle less `(X & 31, Y & 31)`. Cells in row order; in a
   cell the layers 0, 1, 2 in order. The map wraps endlessly; the look-ups of neighbours
   for shadows and table edges do not wrap.
1. Tile numbers: 0 nothing; 1 to 1023 from `bitmaps[5 + id / 256]` (B to E), 32 by 32 at
   x = `(id % 8 + (id / 128 % 2) * 8) * 32`, y = `(id % 256 / 8 % 16) * 32`; 1536 to 2047
   from `bitmaps[4]` (A5) by the same formula; from 2048 autotiles with kind
   `k = (id - 2048) / 48` and shape `(id - 2048) % 48`: A1 (2048 to 2815), A2 (2816 to
   4351, block 64 by 96 at `(k % 8 * 64, (k / 8 - 2) * 96)`, a table when `k % 8 == 7`),
   A3 (4352 to 5887, block 64 by 64 at `(k % 8 * 64, (k / 8 - 6) * 64)`, wall shapes), A4
   (from 5888, row `r = k / 8` at y = `(r - 10) * 80 + (16 if r is odd)`, floor shapes
   for even r and wall shapes for odd r; numbers from 8192 continue the formula). The
   corners of the sixteen A1 blocks and the shape tables are in the report. Source pixels
   outside the bitmap are skipped one by one; a nil or disposed bitmap draws nothing.
   Numbers 1024 to 1535 and negative numbers read outside the original's arrays.
2. Shapes: the tables `FLOOR` (48), `WALL` (16) and `FALL` (4) of the model; wall shapes
   16 to 47 and waterfall shapes 4 to 47 draw the piece at (0, 0) four times.
3. Animation: one counter per tilemap, one more per `Tilemap#update` and by nothing else;
   `step = counter / 30`. A1 floors except kinds 2 and 3: block x plus
   `64 * [0, 1, 2, 1][step % 4]`. Waterfalls (odd kinds from 5): block y plus
   `32 * (step % 3)`.
4. Planes: the lower draws every tile whose entry of `passages` lacks the bit 0x10; the
   upper draws the tiles of layer 2 that have it; tiles of layers 0 and 1 with the bit are
   never drawn. Tables, the edge under a table, the 8-row band rule and the shadows (the
   left 16 by 32 of a cell darkened by `(v * 127) >> 8` under the measured condition on the
   layer-0 tiles at (x-1, y) and (x-1, y-1)) are as section 4 of the report states them.
5. Scrolling: ox and oy of any sign. The viewport's ox and oy only add to the tilemap's.
   Nothing is drawn beyond the 672 by 512 of a plane. `Tilemap#visible = false` hides
   nothing. The viewport's ox and oy are latched at a refresh (`ox=`, `oy=`, `map_data=`,
   `flash_data=`, `passages=`, `bitmaps[]=`, `update`, a change of the viewport's ox or
   oy), not at `viewport=`.
6. Flash: an entry above 0 holds three nibbles r, g, b (bits 12 to 14 ignored);
   `level = 4 + [0, 1, 2, 3, 4, 3, 2, 1][(updates / 4) % 8]`; every channel of the cell
   becomes `min(255, c + nibble * level)`, once with the lower plane and once more with
   the upper; the index wraps into the flash table as in RGSS1.
7. A table with fewer than three layers reads behind its data in the original; a fourth
   layer is ignored; `map_data` nil or empty draws nothing.
8. Changes to the elements of the tables, to bitmap pixels and to the slots show in the
   next frame without `update`.
9. Blend of a tile pixel over the frame: `(s * (a + 1) + d * (256 - a)) >> 8`; when a
   clipped row of a transfer has an odd count of pixels, its last pixel uses the formula
   of a sprite.

## Goals / Non-Goals

**Goals:** a tilemap of a game of RGSS2 is drawn pixel for pixel as the original draws it,
for every case the model covers; the class has the methods of RGSS2. XP unchanged.

**Non-Goals:** the y key in the order of elements of equal z beyond giving the planes
y 0 (it comes with the measurement of sprites); the tilemap under `Viewport#flash`,
a freeze or a transition beyond what follows from being an element.

## Decisions

- One new op, a tilemap plane of RGSS2, with: which plane, the clip, the corner, the
  first cell, the animation step, the flash level, the ids of the map, passages and flash
  tables, and the nine bitmaps. Both compositors evaluate it per pixel from these, as
  they do for RGSS1; nothing is composed in the guest. The shape tables and the tile
  numbering have one definition that both compositors use, as "One definition of the
  tile numbering" asks for RGSS1.
- The blend of rule 9 is the window formula of `frame-composition` with its odd-row rule;
  the op uses that arithmetic. If the rows the original clips differ from the op's clip
  rows (a row of a transfer is a cell or a quarter, not the plane), the op follows the
  original's rows; the cases `alpha1` of the report decide.
- What the original leaves undefined, vkxp defines without reading outside anything:
  numbers 1024 to 1535 and negative numbers draw nothing; a missing layer is empty.
- The scene keeps for a tilemap of RGSS2: the nine slots, the three tables by reference,
  ox, oy, the update counter and the latched viewport origin of rule 5. `visible` is
  stored and has no effect, as measured.
- In RGSS2 the class has `bitmaps`, `passages`, `passages=` and not `tileset`,
  `autotiles`, `priorities`; `Tilemap::BitmapArray` is defined and `TilemapAutotiles` is
  not. Slots outside 0 to 8 read `nil` and store nothing. `map_data = nil` and
  `passages = nil` raise `TypeError` once a table is set. `bitmaps` works on a disposed
  tilemap. Both pending lists of the method test lose these names.
- The protocol version goes up by one; `README.md` names the new version.
- Tests: the synthetic sheets of the report are rebuilt by formula in the tests. Expected
  pictures are committed as digests or small pictures produced by the report's model: every
  tile number range and every shape of each kind, the animation steps, tables, shadows,
  the flash sequence, scrolling with wrap, and forty frames of the random generator. The
  CPU reference is checked against them and the Vulkan backend against the reference.
