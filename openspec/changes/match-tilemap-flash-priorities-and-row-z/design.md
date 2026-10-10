# Design

The rules below were measured on the original runtime (RGSS103J under Wine) with generated
tilesets; every rule was checked by a model against every pixel of the original's frames. The
sections are taken from the measurement report and keep its item numbers; the files they name
(`ph-tilemap-*`) are in `ref/tools/oracle/measurements/` of the workspace, outside the
repository (from a clone under `wt/<name>/`: `../../ref/tools/oracle/measurements/`). The
sample files hold generated patterns only and may be copied into the crate's test data.

## Measured rules

### 1. Priority outside 0..5  (`vkxp-core/src/tilemap.rs:266`)  - DIFFERS

Probe `ph-tilemap-prio-probe.rb`, one run per value (a crash ends a run), samples
`ph-tilemap-prio-samples.txt`. The priorities table holds the value for 8 tile ids from the
start; frame 0 shows no such tile, frames 1.. show rows of them (20 cells per row, rows 0..5 and
every third row), with 1280 one-pixel sprites of z from -1310720 to 1306624, then with no
sprite visible. A second series creates no sprite at all.

| value | with the sprites | without any sprite |
|---|---|---|
| -1, -2, -3, -100, -32768 | crash | (not run) |
| 6, 7, 9 | nothing drawn | nothing drawn |
| 8 | nothing drawn | nothing drawn |
| 10, 12, 16, 20, 24, 32, 50, 64, 99, 100, 128, 255, 256, 1000, 10000 | crash | (not run) |
| 32766, 32767 | nothing drawn | crash |
| 5 (control) | drawn, z as item 4 | drawn |

"Crash" = `wine: Unhandled page fault on read access to 10003B00 at address 10003B00` (the same
address every time) on the first `Graphics.update` that has such a tile in the visible area; a
Ruby exception looks different (message box), so this is a native crash. The value merely
standing in the table for a tile that is not on the map is harmless. "Nothing drawn" = the
cells are empty at every z (black with the sprites hidden; nothing ever in front of or behind a
sprite). 38 runs. Because the same value (32766, 32767) crashes or not depending on whether
sprites exist, the original evidently reads memory out of bounds; there is no rule to copy for
those values.

vkxp today clamps: a negative priority is drawn as ground (priority 0), a value above 5 as
priority 5. Frames 1..5 of the runs for 6, 7, 8, 9, 32767 differ in 92448..174096 pixels (vkxp
draws the tiles, the original draws nothing).

What to change: a tile whose priority is not in 0..5 must not be drawn at all (neither ground
nor row strip). That reproduces the original wherever the original survives (6..9 always,
32766/32767 sometimes) and is the only measured non-crashing behaviour.

    priorities[id]   expected
    0                ground, z 0
    1..5             row strip, z as item 4
    6, 7, 8, 9       not drawn
    <0 or >=10       original crashes (or draws nothing); vkxp: not drawn

### 2. flash_data  (`vkxp-core/src/tilemap.rs:291`)  - DIFFERS

Probes `ph-tilemap-anim-probe.rb` (160 consecutive frames), `ph-tilemap-z2-probe.rb` (z order,
high bits), `ph-tilemap-flashsize-probe.rb` (40 frames, table of another size than the map).
Samples `ph-tilemap-flash-samples.txt`. Checks `ph-tilemap-flash-check.py` (19,660,800 pixels,
0 mismatches apart from the cell holding -1, see below) and `ph-tilemap-flashsize-check.py`
(40 whole frames, 0 mismatches).

Rule. Let `u` = number of `Tilemap#update` calls made on THIS tilemap since it was created
(a second tilemap starts at 0 on its own; assigning a new flash table does not reset it;
`Graphics.update` without `Tilemap#update` does not advance it; two updates in one frame
advance it by two).

    tri  = [0, 1, 2, 3, 4, 3, 2, 1]
    m(u) = 4 + tri[(u / 4) % 8]                 # 4..8, period 32 updates, each value held 4 updates
    v    = flash_data[x, y]  (signed 16 bit)
    v < 0  (bit 15 set): nothing is drawn        # -1, -2, 0xf123, 0x8123, 0xf000, 0x8fff measured
    v >= 0: r = (v >> 8) & 15, g = (v >> 4) & 15, b = v & 15     # bits 12..14 are ignored
            for the 32x32 pixels of the cell, per channel:
            screen = min(255, screen + nibble * m(u))             # additive, no alpha involved

    u       0-3  4-7  8-11  12-15  16-19  20-23  24-27  28-31  32-35 ...
    m(u)     4    5    6     7      8      7      6      5      4
    0xfff   +60  +75  +90   +105   +120   +105   +90    +75    +60     (on every channel)
    0x888   +32  +40  +48   +56    +64    +56    +48    +40    +32

Before the first update (u = 0) the flash is already visible with m = 4. Samples (u, value,
destination -> screen): (1, 0xfff, (0,255,0)) -> (60,255,60); (4, 0x7abc, (50,100,150)) ->
(100,155,210); (4, 0xfff, (50,100,150)) -> (125,175,225); (4, -1, ...) -> unchanged;
(1, 0x1000, d) -> d; (44, 0xf00, (204,51,110)) -> (255,51,110) (m = 7, clamped).

What it is drawn on: the flash is one layer at z = 0, drawn directly after the ground
(priority 0) tiles of the tilemap, with the tie rule of item 4. So it brightens the ground
tiles and autotiles, a cell that holds no tile at all (measured over black: the pure
`nibble*m`), and ALSO sprites of z < 0 and sprites of z = 0 created before the tilemap (they
lie below it: magenta (255,0,255) sprite at z <= 0 over an empty flashing cell gives
(255,60,255)). It does not show through tiles of priority 1..5 (they are above, z >= 64 + ...)
nor over sprites of z > 0. Tilemap#oy does not change that (z stays 0).

Which flash cell a screen cell uses (only matters when the flash table is not the size of the
map; equal sizes give the map cell): with floor division and non-negative remainders,

    c0 = floor(ox / 32) mod map_xsize            # the first column, wrapped by the MAP width
    flash_x(i) = (c0 + i) mod flash_xsize        # i = 0, 1, .. counts screen columns from the
                                                 # column that contains pixel ox
    same for y with oy, map_ysize, flash_ysize

    map 7x5, flash 5x4:  ox=0   -> flash x of screen columns: 0 1 2 3 4 0 1 2 ...
                         ox=40  -> 1 2 3 4 0 ...        (c0 = 1)
                         ox=264 -> 1 2 3 4 0 ...        (c0 = 8 mod 7 = 1, NOT 8 mod 5 = 3)
                         ox=-50 -> 0 1 2 3 4 ...        (c0 = -2 mod 7 = 5, 5 mod 5 = 0)
                         ox=-300 (c0 = -10 mod 7 = 4) -> 4 0 1 2 3 ...
    map 7x5, flash 9x6:  ox=-300 -> 4 5 6 7 8 0 1 ...   ; oy=-200 (r0 = -7 mod 5 = 3) -> 3 4 5 0 1 ...

(The sample file lists the first WHOLE cell first, which is the second column when ox is not a
multiple of 32.) Flash tables 5x4, 9x6, 3x7, 7x5, 1x1 at 8 origins each: 0 mismatches; the
simpler model "map cell mod flash size" is wrong in 18 of the 40 frames.

vkxp today: additive too, but colour = nibble*17 with alpha `flash_alpha(u)` = a triangle
0..255 over 32 updates (0 at u = 0, so no flash before the first update; peak 255 = +255 for
nibble 15 where the original peaks at +120), negative values are drawn from their low 12 bits,
and the flash cell is the map cell modulo the flash size. All 160 + 40 frames differ.

What to change:
- `flash_alpha` -> the multiplier m(u) above; fill the overlay cell with colour
  (r*m, g*m, b*m) and alpha 255 and keep the additive blend (with alpha 255 the additive sprite
  formula adds the colour exactly); skip entries < 0 (and entries whose three nibbles are 0).
- index the flash table with (c0 + i) mod flash size, c0 wrapped by the map size.
- z order is already right (`(0, Strip::Flash)` after the ground).
- Not measured: a flash table with ysize 1 / zsize > 1 / xsize 0; the pulse beyond 141 updates
  (more than 4 periods seen, no drift).

### 4. z of the tiles  (`vkxp-core/src/tilemap.rs:486`)  - DIFFERS in two details

Probes `ph-tilemap-z-probe.rb` (16 frames) and `ph-tilemap-z2-probe.rb`, samples
`ph-tilemap-z-samples.txt`, check `ph-tilemap-z-check.py` (15 frames, 4,587,520 pixels, 0
mismatches for the original). 640 sprites one pixel wide with z = x + offset stand in the
tilemap's viewport, half of them (set A) created before the tilemap, half (set B) after; every
map row holds one tile id, priorities 0..5 in two different row patterns; oy = 0, 7, 16, 20,
31, 32, 45, 109, -20, 300; Viewport#oy 0 and 10; Viewport#rect.y 0 and 16; also with no
viewport at all.

Rule.
- Tiles of priority 0 (all three layers) and the flash layer: z = 0.
- A tile of priority p = 1..5 in map row R (R counted without wrapping: R = floor((Yv + oy)/32)
  for a pixel at viewport-local y Yv before Viewport#oy is applied, see next point):

      z = 32*R - tilemap.oy + 32 + 32*p

  i.e. "y of the row's top edge, plus 32, plus 32 per priority step", where the y of the top
  edge is taken from the tilemap origin ONLY. Viewport#oy scrolls the picture but does not
  change z; Viewport#rect does not change z; without a viewport the same formula holds.
- Tie between a tile (or the flash layer) and a sprite of the same z in the same viewport:
  the one created later is on top. A sprite created before the tilemap is below the tile, a
  sprite created after the tilemap is above it. Assigning `sprite.z` later (to another value
  and back, or directly to the tie value) does NOT change that order; neither does assigning
  a new `map_data`, `oy`, or `Tilemap#update`.

      R  p  oy   z            R  p  oy   z
      0  0  any  0            1  1   7   89
      1  1   0   96           1  1  31   65
      2  2   0   160          2  2  32   128
      3  3   0   224          3  3  109  115      (row partly above the screen)
      4  4   0   288          -1 5  -20  180      (map 20x4, the row above row 0)
      5  5   0   352          11 5  300  244
      7  1   0   288          1  1   0   96 with Viewport#oy = 10 (vkxp gives 86)
      14 2   0   544          1  1   7   89 with Viewport#oy = 10, rect.y = 16 (vkxp gives 79)

  Sprite against the priority-1 tile of row 1 at oy 0 (tile z 96): sprite z 95 below, z 97
  above, z 96: below if the sprite was created before the tilemap, above if after.

vkxp today: formula and ground z agree (frame 0 and the two frames without viewport: 0
differing pixels). Two differences:
1. `strips(scroll)` subtracts the viewport's oy from the row top (`top = ... - oy - scroll`):
   with Viewport#oy = 10 every priority row is 10 too low in z (frames 11 and 13: 4180 and 4080
   differing pixels). Change: do not subtract `scroll` when computing z (it still positions the
   picture).
2. After `sprite.z = ...` is assigned again, a sprite created before the tilemap wins the tie
   against tiles in vkxp (240 pixels = one sprite column per row in frames 1..10, 12; the
   same in the z2 and prio runs). In the original the order among equal z stays the creation
   order whatever is assigned later. Change: the tie-break key must be the creation sequence
   of the object (sprite / tilemap), not the time of the last z assignment.
   Not measured here: ties among sprites, planes or windows themselves.

### Confirmed, nothing changes

- Item 3: the map repeats beyond its edges in both directions, priority tiles included.
- Item 5: tile ids 1 to 47 draw nothing.
- Item 6: autotile sheets of unusual sizes are drawn as `vkxp_proto::tiles` does today.
- Item 7: a table element keeps the low 16 bits.

## Decisions

- A tile whose priority is not 0 to 5 is not drawn. Where the original crashes there is
  nothing to match; not drawing is the only behaviour it shows when it survives.
- The flash overlay keeps its place (z 0, after the ground) and its additive blend; its
  colour becomes `nibble * m(u)` per channel at full alpha, entries below 0 are skipped, and
  the overlay is there before the first update.
- The flash cell of a screen cell is found from the first column and row wrapped by the map's
  size, then by the flash table's size.
- The z of a priority row no longer includes the viewport's origin.
- The tie between a tile and a sprite of the same z (difference 2 of item 4) is settled by
  the change `match-draw-order-self-transfer-and-colour-wrap`, which fixes the stamp at
  creation; this change does not touch stamps.
- The notes this change settles or confirms lose the word `PLACEHOLDER` (in
  `crates/vkxp-core/src/tilemap.rs` and `crates/vkxp-proto/src/tiles.rs`, comments only in
  the latter).

Out of the measurements but not done here: the conversion of table values outside the range
of a 32-bit integer (the original raises `RangeError`), `Table#[]=` with one index fewer than
the dimensions, and the original's unchecked `Table._load`.

## Follow-ups

- `Tilemap::strips` keeps its `scroll` parameter, now unused, because its only caller is in
  `crates/vkxp-core/src/extract.rs`, which other changes edit at the same time. Removing the
  parameter, the two lines there that compute it and the comment above them ("Rows rank by
  where they are in the viewport, which its origin scrolls") is left to a later change.
