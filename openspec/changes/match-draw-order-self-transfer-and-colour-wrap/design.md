# Design

The rules below were measured on the original runtime (RGSS103J under Wine) with generated
bitmaps; "0 wrong" means a model of the rule predicts every pixel of every probe frame. The
sections are taken from the measurement report, whose item numbers they keep; file names in
them (`ph-scene-*`, `ph_scene_*`, `README.md` of the measurements) are in
`ref/tools/oracle/measurements/` of the workspace, outside the repository (from a clone under
`wt/<name>/`: `../../ref/tools/oracle/measurements/`). The sample files there hold generated
patterns only, no game or run-time package art, and may be copied into the crate's test data
as test vectors.

## Measured rules

### 4. Draw order among elements with equal z (scene.rs:189, extract.rs:397 `sort`)

Probe `ph-scene-zorder-probe.rb`: 96 cases in 64x48 cells (13 of them observed in two
frames, before and after a `Graphics.update`: 109 observations), every pair of elements of a
case read separately; plus a tilemap frame and an allocation frame.

Rule (0 of 109 observations mismatch; every pair of every case):

    elements are drawn in ascending order of (z, creation order): among equal z the element
    that was CREATED later is in front. Nothing else takes part.

In particular none of these changes the order among equals: assigning z the same value;
assigning z another value and back (from above or from below, before or after the element was
first drawn, in any interleaving: cases 2..12, 20..31, 94, 95); `y` (cases 13, 14: no y
tie-break in this runtime); `visible = false` then `true`; assigning the bitmap again;
disposing an older element. Rejected rules: "last z assignment that changed the value" (vkxp
today) mismatches 42 observations, "last z assignment" 46, "list in which an element moves
only as far as needed when its z changes" 50.

The same key applies to every kind and level:
- Sprite, Plane: (z, creation).
- Window: lower layer (z, creation of the window), upper layer (z + 2, creation of the
  window) - the README rule; now separated from "last z assignment" (cases 39..48: a window or
  a sprite taken to another z and back keeps its creation rank).
- Viewport: one place among the top-level elements with (viewport.z, creation of the
  VIEWPORT) - not of its contents (cases 59..61, 68, 69: `v V; s B; s A V` draws top-level B
  over A although A is younger; `s B; v V; s A V` draws A over B). A viewport z taken away and
  back keeps the creation rank (62..66, 70..74, 84..86). Inside a viewport the elements are
  ordered by (z, creation) among themselves (56..58, 79, 80).
- Tilemap ground layer (priority 0, z 0): a sprite with z 0 is behind it if created before
  the tilemap and in front if created after; z 5 -> 0 or -5 -> 0 after the tilemap exists does
  not move a sprite created before it in front (squares 1 and 7 stay behind). Same inside a
  viewport. (All "after" squares were created after `Tilemap.new` and after its tileset and
  map were assigned, so "creation of the Tilemap" versus "assignment of its data" was **not
  determined**.)
- "Creation order" is not the address of the object: 20 sprites created after 20 others had
  been disposed and garbage collected (so that memory was free for reuse) all drew in front of
  older sprites (frame 3, 20 of 20).
- A viewport change cannot be tested: `Sprite`, `Plane` and `Window` have no `viewport=` in
  this runtime (`method_defined?` is false for all three). Default z of Sprite, Plane, Window
  and Viewport is 0.

vkxp today: **differs**: it renews the stamp on a z assignment that changes the value
(42 of the 109 observations differ, exactly the ones the "zchange" rule mispredicts; and the
tilemap squares 1 and 7). Cases without a z round trip are equal.

What must change: the stamp of an element (sprite, plane, window, viewport, tilemap) is given
once, at creation, and never renewed; `sort` stays `(z, stamp)`; the doc comment of `Scene`
("or last given a different z, more recently is in front") becomes "created more recently is
in front".

Unit-test table (all elements start at z 0; bottom -> top):

| ops | original order | vkxp today |
|---|---|---|
| new A, new B, new C | A B C | A B C |
| new A, B, C; A.z = 5; A.z = 0 | A B C | B C A |
| new A, B, C; C.z = -5; C.z = 0 | A B C | A B C |
| new A, B, C; B.z = -5; B.z = 0 | A B C | A C B |
| new A, B, C; C.z = 10; B.z = 10; A.z = 10 | A B C | C B A |
| new A, B, C, D; A.z = 3; B.z = 3; C.z = 3 | D A B C | D A B C |
| new A, B, C, D; D.z = -3; C.z = -3; B.z = -3 | B C D A | D C B A |
| new A, B, C; update; A.z = 5; A.z = 0 | A B C | B C A |
| plane A, sprite B; A.z = 5; A.z = 0 | A B | B A |
| window A, window B; A.z = 5; A.z = 0 | contents of A below contents of B | A above B |
| viewport V, viewport W, sprite A in V, sprite B in W; V.z = 5; V.z = 0 | A B | B A |
| viewport V, sprite A in V, sprite B (top level); V.z = 5; V.z = 0 | A B | B A |
| viewport V, sprite B (top level), sprite A in V | A B (the viewport is older than B) | A B |
| sprite B (top level), viewport V, sprite A in V | B A | B A |
| sprite S (z 0), Tilemap, then S.z = 5; S.z = 0 | S behind the ground tiles | S in front |

Side note: `Viewport#disposed?` raised `NoMethodError` in the original (first run of the
probe). Whether vkxp defines it was not checked.

### 6. Fractional Color / Tone components (color.rs:145 `Color::to_proto`, :261 `Tone::to_proto`)

Probe `ph-scene-fraction-probe.rb`: 18 uses x 26 values x 2 frames = 936 blocks. Uses: sprite
colour r/g/b (alpha 255 and 128), sprite colour alpha, sprite tone r/g/b, sprite tone grey,
viewport colour r/g/b and alpha, viewport tone r/g/b and grey, sprite flash r/g/b and alpha,
viewport flash r/g/b and alpha, plane colour r/g/b and alpha, plane tone, and two rows with all
components fractional at once. The objects keep the fractions (`Color.new(127.5,-0.5,254.9,0.99)`
reads back unchanged).

Rule (every block consistent):

    every component is truncated toward zero when it is used
    Color red/green/blue: then taken modulo 256 (low byte of the integer): -1 -> 255, -5 -> 251,
                          -100.7 -> -100 -> 156; -0.5 and -0.9 -> 0
    Color alpha:          below 0 -> 0   (-1, -1.5, -5, -100.7 all give no effect)
    Tone red/green/blue:  truncated toward zero (-27.5 and -27.9 -> -27, -0.99 -> 0), no wrap
    Tone grey:            truncated (126.5 -> 126, 254.9 -> 254); negative -> 0
    flash: the alpha is truncated when the flash starts; the decay then works on the integer
           (127.9, duration 10, one update: 127*9/10 = 114, not 115; 2.5 -> 2 -> 1)

| value | sprite colour (v,v,v,255) on grey 100 | (255,255,255,v) on black | tone (t,t,t,0) on grey 100 |
|---|---|---|---|
| 0.5 / 0.9 / 0.99 | 0 | 0 | 100 (t = 0.5, 0.9) |
| 1.5 | 1 | 1 | 101 |
| 126.5 | 126 | 126 | - |
| 127.5 / 127.9 | 127 | 127 | 127 (t = 27.5, 27.9) |
| 128.4 | 128 | 128 | - |
| 254.5 / 254.9 | 254 | 254 | 254 (t = 154.5) |
| -0.5 / -0.9 | 0 | 0 | 100 |
| -1.5 | **255** | 0 | 99 |
| -5 | **251** | 0 | - |
| -100.7 | **156** | 0 | 0 (t = -100.5) |
| -27.5 / -27.9 | - | - | 73 |

Tone grey on a (255,0,0) sprite: 126.5 -> (166,37,37) = grey 126; 127.5 and 127.9 ->
(165,38,38) = grey 127; 128.4 -> (164,38,38); 254.5 and 254.9 -> (76,75,75) = grey 254;
0.5, 0.9, 0.99, -0.5, -3 -> unchanged.
Viewport colour/tone/flash, plane colour/tone and sprite flash give the same numbers as the
sprite rows (see the samples file).

vkxp today: **matches for every fractional value** (truncation is right; the two PLACEHOLDER
notes can be replaced by the provenance). It differs only for Color red/green/blue <= -1
(values -1, -1.5, -5, -100.7: 56 of the 936 blocks, in sprite colour, viewport colour, both
flashes and plane colour), where `to_proto` clamps to 0 and the original wraps.
What must change: in `Color::to_proto` red/green/blue = `(component.trunc() as i64) as u8`
(wrapping, as `Color::to_rgba`'s `wrap` already does for bitmaps); alpha stays
"truncate, negative is 0". Measured down to -100.7 only; values below -255 were not measured.

### 8. `blt` / `stretch_blt` from a bitmap onto itself (bitmap.rs:474)

Probe `ph-scene-selfblt-probe.rb`: 12x12 bitmap, 17 shifts (all four directions by 1 and 3,
the four diagonals, (1,3), (-3,1), (5,5), (-5,-5), (0,0)) x 11 operations (blt opaque/alpha
bitmap at opacity 255 and 128; stretch_blt same size, 2x up, 2x down, at 255 and 128), whole
bitmap shifted by one pixel in the four directions, and a mirrored (negative width) blt: 197
cases, every pixel dumped with `get_pixel`.

Rule (0 of 197 cases mismatch; model `ph-scene-selfblt-model.py`):

    the transfer is done in place, with no temporary copy:
    for j in 0 .. dh-1 (top to bottom):
        for i in 0 .. dw-1 (left to right):
            dst(dx + i, dy + j) = blend( dst(dx + i, dy + j), src(sx(i), sy(j)), opacity )
    where src() reads the bitmap AS IT IS AT THAT MOMENT, i.e. including the pixels this same
    transfer has already written; sx(i), sy(j) are the usual blt / stretch_blt source indices
    (mirror: sx(i) = rect.x - 1 - i), blend is the usual blt blend, clipping as usual.

Rejected: a snapshot of the source (vkxp today): 119 cases mismatch; bottom-to-top rows: 139;
right-to-left columns: 86; both reversed: 169.

Consequences, 1-D illustration that follows from the rule (row of pixels `a b c d e f`,
opacity 255; the measured 12x12 dumps behave exactly so):

| call | original | snapshot (vkxp today) |
|---|---|---|
| blt(1, 0, self, Rect(0,0,4,1)) (shift right by 1) | a a a a a f | a a b c d f |
| blt(2, 0, self, Rect(0,0,4,1)) (shift right by 2) | a b a b a b | a b a b c d |
| blt(0, 0, self, Rect(1,0,4,1)) (shift left by 1) | b c d e e f | b c d e e f |
| shift down by 1 (rows r0 r1 r2 r3, Rect of 3 rows) | r0 r0 r0 r0 | r0 r0 r1 r2 |
| shift up by 1 | r1 r2 r3 r3 | r1 r2 r3 r3 |

So shifts to the left or upwards equal the snapshot result; shifts to the right or downwards
smear the first column / row (period = the shift distance). With opacity < 255 each written
pixel is a blend and is read again later, so the smear fades progressively; stretch_blt
behaves the same way with its index walk (an enlargement around its own centre reads partly
written pixels). Measured rows from the samples file (base pixel (x, y) =
(x*20+5, y*20+5, x+y*12), `blt(4, 3, b, Rect(3,3,6,6))`, i.e. shift right by 1), row 3,
x = 0..11, rrggbbaa:

    original: 054124ff 194125ff 2d4126ff 414127ff 414127ff 414127ff 414127ff 414127ff 414127ff 414127ff cd412eff e1412fff
    snapshot: 054124ff 194125ff 2d4126ff 414127ff 414127ff 554128ff 694129ff 7d412aff 91412bff a5412cff cd412eff e1412fff
    opacity 128, original: ... 414127ff 4a4127ff 594127ff 6a4128ff 7d4129ff 90412aff a4412bff cd412eff e1412fff

vkxp today: **differs** in 119 of the 197 cases. blt and same-size stretch_blt: differs for
the shifts (1,0) (0,1) (3,0) (0,3) (2,2) (-2,2) (1,3) (-3,1) - every shift with a component
downwards, or to the right within the same rows - at both opacities and for both bitmaps, and
for the mirrored blt; equal for (-1,0) (0,-1) (-3,0) (0,-3) (-2,-2) (2,-2) (0,0) and for the
shifts (5,5) (-5,-5), whose rectangles barely overlap. stretch_blt 2x up and 2x down: differs
for 13 of the 17 shifts, including (0,0); equal only for (0,-3) (-2,-2) (5,5) (-5,-5).
What must change: `Bitmaps::transfer` with `src == dst` must not read from a copy; it walks
destination rows top to bottom and columns left to right and reads each source pixel from the
bitmap in its current state. (With `src != dst` nothing changes.) The 197 dumps of
`ph-scene-selfblt-samples.txt` are the test vectors: each is a name line (operation, bitmap
kind, opacity, shift) and 12 rows of 12 `rrggbbaa` pixels; the base bitmaps and the exact
calls are in the probe.

### 1. Negative zoom (confirmed, nothing changes)

A sprite at angle 0 with a zoom of 0 or less draws nothing; at another angle the rule for
turned sprites applies with the signed zoom (96 cases, 0 differing pixels against vkxp). A
plane with a negative zoom makes the original hang in `Graphics.update`; vkxp draws nothing
for it and goes on, which is kept.

## Decisions

- The stamp of an element (sprite, plane, window, viewport, tilemap) is given once, when it
  is created, and never renewed; the sort key stays `(z, stamp)`.
- A tilemap's rank is its creation. Whether the original ranks it by the assignment of its
  data instead was not determined.
- `Color::to_proto` wraps red, green and blue as `Color::to_rgba` already does for bitmaps.
  Components below -255 cannot occur once colours clamp at -255.
- A transfer within one bitmap reads the live pixels; the code path for two different
  bitmaps is untouched.
- The notes at `Geometry::transform`, `Geometry::turned_box`, `Color::to_proto`,
  `Tone::to_proto`, `Scene`, `sort` and `Bitmaps::blt` lose the word `PLACEHOLDER` and name
  the measurement.
- The two formulas `blend_add_translucent` and `blend_sub_translucent` of `vkxp-pixel` stay
  as they are: the original has no translucent destination for them, so there is nothing to
  match, and frame composition never reaches them.

## Edits outside the named files

- Draw order: three tests outside the named files stated the old order and were corrected
  in the lines that stated it: `tilemap/tests.rs`
  (`a_tile_of_priority_1_ranks_with_the_sprites_one_row_below_it`: at the z of the tile the
  tilemap, created after the sprite, is in front) with its line of
  `tests/data/tilemap-frames.txt`, recorded again (its third digest is now that of the frame
  with the tile in front); `window/tests.rs` (`a_sprite_can_stand_between_frame_and_contents`:
  a window z taken away and back leaves the younger sprite in front of the contents); and the
  sentence on the draw order in the crate documentation of `lib.rs`.

## Follow-ups

(none yet)
