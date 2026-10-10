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

### 2. Turned sprite inside a viewport (turned.rs:67 `turned_placement`)

Probes: `turnedvp` (32 cases: angles 30/90/180/45/200, viewport ox/oy (0,0) (13,7) (-9,-5)
(100,-50), two rect origins, sprites cut by each viewport edge), `geom2` frame 2 (12 cases:
viewport partly off screen at the left and at the top, mirror + src_rect, negative zoom, ox only,
oy only, angle 360) and 4 cases of `bush` frame 2: 48 turned sprites in viewports, 0 wrong
pixels.

Rule. Let the viewport have rect (vx, vy, vw, vh) and origin (vox, voy); the sprite x, y, ox,
oy, zoom, angle, src_rect (sx, sy, w, h) as usual; `s`, `c` as in the README correction
(`t = ((-angle)+(-angle))*pi/360`, `s = sin t`, `c = cos t`). All arithmetic in doubles.

    xa = x - vox ; ya = y - voy                    (position used for the bounding box only)
    P0 = ( xa - (ox*c*zx - oy*s*zy),  ya - (ox*s*zx + oy*c*zy) )
    P1 = ( w*c*zx + P0.x, w*s*zx + P0.y )   P2 = ( P0.x - h*s*zy, h*c*zy + P0.y )
    P3 = ( w*c*zx + P2.x, w*s*zx + P2.y )
    left/right = min/max of floor(P.x + 0.5), top/bottom = min/max of floor(P.y + 0.5)
    for j in top .. bottom-1:
        U = (sx + ox) + ((left - x)*c + (j - y)*s) / zx          <- the RAW x and y, not xa, ya
        V = (sy + oy) + ((-s)*(left - x) + (j - y)*c) / zy
        mirror: U = (sx + w - 1) - (U - sx)
        for i in 0 .. right-left-1:
            if sx <= U < sx+w and sy <= V < sy+h:
                screen pixel (vx + left + i, vy + j) shows texel (trunc U, trunc V),
                if that pixel lies inside the viewport rectangle (and the screen)
            U = c/zx + U  (mirror: U = U - c/zx) ;  V = (-s)/zy + V

So the box sits where it should (the sprite's box at screen position
(vx + x - vox, vy + y - voy)), but the picture inside it is the picture of a sprite standing at
(vx + x, vy + y): displaced by (+vox, +voy) against its box, cut where it leaves the box, and
the part of the box it leaves uncovered is empty. With vox = voy = 0 this is the ordinary
turned sprite moved by (vx, vy) (the rect origin alone changes nothing). With a large origin
the picture leaves its box entirely and nothing is drawn (8 cases with vox = 100, voy = -50:
nothing drawn although the sprite "should" be in view). The viewport rectangle clips; it does
not shift the box (viewport at x = -30 or y = -20: 0 wrong). angle 0 in the same viewport is
placed normally (control, 0 wrong).

vkxp today: places the turned sprite as if at (vx + x - vox, vy + y - voy) with a consistent
picture. 0 differing pixels for vox = voy = 0, **differs for every case with a non-zero
viewport origin** (some hundreds up to 2025 differing pixels per case; where the original draws nothing vkxp
draws the whole sprite).

What must change (turned.rs `turned_placement` / `turned_row`): keep the box as it is computed
now, but compute it in viewport coordinates from `(x - vox, y - voy)` and add `(vx, vy)`
afterwards (identical integers; doing the float arithmetic at the same magnitudes as the
original keeps the knife-edge cases of 90/180 degrees equal, which is how the model was
verified), and evaluate `U`, `V` with `left`/`j` in viewport coordinates and the raw
`self.x`, `self.y`. The clip stays the viewport rectangle.

Worked example (case turnedvp/1; measured, model agrees): pattern 24x16, sprite x=68 y=47
ox=12 oy=8 zoom 2x2 angle 30, viewport rect (180,15,110,80), ox=13 oy=7.

    box from (55, 40): left 26, top 14, right 84, bottom 66 (viewport coordinates)
        = screen [206,29)..(264,81); vkxp draws 2025 pixels in it
    original: 1270 pixels, bounding box of the drawn ones [220,37,264,81)
    screen row 59, x = 220..263:
      . . 1,0 1,0 2,0 2,0 3,1 3,1 4,1 4,1 4,2 5,2 5,2 6,2 6,3 7,3 7,3 7,3 8,4 8,4 9,4 9,4 10,5 10,5
      11,5 11,5 11,6 12,6 12,6 13,6 13,7 14,7 14,7 14,7 15,8 15,8 16,8 16,8 17,9 17,9 17,9 18,9 18,10 19,10
    screen (240,60) shows texel (9,5); (250,70) shows (10,11); (230,45) and (220,58) show nothing
    same sprite with viewport ox=-9 oy=-5 and x=46 y=35 (case turnedvp/2, viewport rect at (340,15)):
      same box; screen (380,45) shows (10,4), (400,60) shows (15,15), (414,75) nothing

For a unit test without Wine: build the expected picture with the loop above (it is the
function `predict_case` of `ph_scene_geom.py`, option `tv=orig`), or take the three rows per
case from `ph-scene-geom-samples.txt` (topics turnedvp, geom2 cases 32..47, bush cases 40..43).

---

### 3. bush_depth on a turned sprite (turned.rs:194 `render_turned`, extract/tests.rs:925)

Probes: `bush` (angles 30, 90, 180, 45, 270, 360, 135, 1; depths 1, 3, 4, 8, 16, 20, 40, 100,
-3; zooms 1x1, 2x2, 3x1.5 mirrored, 2x3 with src_rect (4,2,14,10); 4 in a viewport), `geom2`
frame 2 (box cut by the viewport bottom, by the screen bottom, by the screen's right edge):
47 turned sprites with bush_depth > 0 and 4 with -3, 0 wrong pixels. Rejected alternatives:
"faint where the texel row is one of the lowest `bush_depth` rows of src_rect" (vkxp today):
18273 wrong pixels in `bush`; "no effect": 13415 wrong.

Rule: with the box rows `top .. bottom-1` of item 2 / the README correction,

    screen row j of the turned sprite is drawn with opacity >> 1 when j >= bottom - bush_depth,

for every pixel of that row, whatever texel it shows; all other rows at the full opacity.
`bottom` is the bottom of the unclipped box (in a viewport: of the box in viewport
coordinates), so a box cut by the viewport or the screen loses faint rows rather than getting
new ones. bush_depth <= 0: no faint rows. bush_depth >= bottom - top: every row is faint
(depth 100 on a box of 52 rows; depths 1, 16, 40 on the same box: 0 wrong). zoom_y does not
scale the depth. The probes used opaque texels and opacity 255, where a faint pixel is
(c*128)>>8 over black, i.e. opacity >> 1 = 127 as in the README bush rule; halving the texel
alpha (vkxp's present way of marking faint rows) gives the same value there. Other alphas
and opacities were not measured for turned sprites.
(The box includes rows that hold no pixel of the sprite, e.g. row `top` in the example, so the
number of visibly faint rows can be smaller than bush_depth.)

This is consistent with the README's remark that the turned pixels go to a temporary bitmap
that is then drawn like an unrotated sprite of the box's size.

vkxp today: **differs** (halves the alpha of the texels of the lowest source rows, so the
faint band turns with the sprite): 12..1228 differing pixels per case; the one case
that agrees is angle 360 at zoom 1 (bush/20), where both rules pick the same rows.

What must change: in `render_turned` drop `faint_from`/`turned_bush` on `v`; instead make
faint every pixel of rows `j >= bounds.bottom - bush_depth`. The test at `extract/tests.rs:925`
should expect: sprite turned by 180 degrees, bush_depth d -> the LOWEST d screen rows of the
box are faint (with 180 degrees these show the TOP texel rows of the source).

Example (case bush/1, measured): pattern 24x16, x=240 y=60 ox=12 oy=8 zoom 2x2 angle 30,
bush_depth 8, no viewport. Box: left 211, top 34, right 269, bottom 86. Rows 35..77 are drawn
at full opacity, rows 78..85 faint (colour (c*128)>>8 over black), row 34 holds no pixel.

    row 78 (first faint row) holds 18 pixels, row 85 holds 2; row 77 (20 pixels) is full.

#### 3b. Unrotated zoomed sprite with bush_depth (side finding, corrects README "bush_depth")

The control cells showed that the README rule ("the lowest bush_depth screen rows are drawn
with opacity >> 1") gets the faint rows right but not their texels when zoom_y != 1. Probes
`geom2` frame 1 and `bushzoom` (32 cases, zoom_y 0.5, 0.7, 1.3, 1.5, 1.9, 2, 2.2, 2.5, 3, 3.7;
depths 1..40), plus the 4 controls of `bush`: 0 wrong pixels with

    h = src_rect height, dh = floor(f32(h * f32(zoom_y)))       (as before)
    bd = min(bush_depth, dh)             (bush_depth <= 0: no bush, the ordinary sprite)
    sb = bd * h / dh                     (integer division: the source rows of the faint part)
    upper part: source rows [0, h - sb)  stretched into screen rows [0, dh - bd)   full opacity
    lower part: source rows [h - sb, h)  stretched into screen rows [dh - bd, dh)  opacity >> 1
    each part with the stretch rule on its own:  v = (j * ((n_src << N) / n_dst)) >> N
    sb == 0: the lower bd rows are not drawn at all (the sprite is bd rows shorter)
    h - sb == 0: no upper part

(`sb = floor(bd / zoom_y)` is rejected by zoom 1.3 depth 5 and 10, zoom 0.7 depth 9, zoom 1.9
depth 15.) The x axis is not affected. With zoom_y == 1 this is the old rule.

Measured texel rows v per screen row, top to bottom (`*` = faint), h = 16:

| zoom_y | depth | rows drawn | v per screen row |
|---|---|---|---|
| 2 | 1 | 31 | 0 0 1 1 2 2 ... 13 13 14 14 15 (16 source rows into 31; no faint row) |
| 2 | 3 | 32 | 0 0 1 1 ... 12 12 13 13 14 15* 15* 15* |
| 2 | 5 | 32 | 0 0 1 1 ... 12 12 13 14* 14* 14* 15* 15* |
| 3 | 2 | 46 | 0 0 0 1 1 1 ... 7 7 7 8 8 9 9 9 ... 14 14 14 15 15 (no faint row) |
| 3 | 4 | 48 | 0 0 0 ... 13 13 13 14 14 15* 15* 15* 15* |
| 1.5 | 3 | 24 | 0 0 1 1 2 3 3 4 5 5 6 7 7 8 9 9 10 11 11 12 13 14* 14* 15* |
| 0.5 | 3 | 8 | 0 2 4 6 8 10* 12* 14* |
| 2.5 | 6 | 40 | 0 0 0 1 1 2 2 2 3 3 4 4 4 5 5 6 6 6 7 7 8 8 9 9 9 10 10 11 11 11 12 12 13 13 14* 14* 14* 14* 15* 15* |

vkxp today: implements the old rule; differs in 48..1152 pixels per case of these two frames
(31 of the 32 cases).
This is not one of the eight placeholders; it is reported because it was found on the way and
`bush_depth` on zoomed character sprites is common.

### 5. Zoomed sprite whose src_rect reaches outside its bitmap (extract.rs:237 `stretched`)

`Sprite#src_rect=` keeps the rectangle as given (read back unchanged, e.g. -6 0 24 16); it is
not clamped to the bitmap.

#### angle == 0

Probes `srcrect` frames 0..3, 5, 6 (rects leaving the 24x16 bitmap on the left, right, top,
bottom, on all sides, far outside, by one texel; zooms 2x2, 1.5x1.5, 2.5x3, 0.5x0.5, 3.3x0.7,
1x1, 3.3x2.7, 0.5x0.4; mirror; ox/oy) and `srcrect2` (one axis at zoom 1): 112 cases, 0 wrong
pixels. Rejected: the plain stretch rule with the outside texels left out (`cut=pure`): 7847
wrong pixels in `srcrect`, 316 in `srcrect2`.

Rule, per axis (x shown; w = src_rect.width, s0 = src_rect.x, bw = bitmap width,
`dw = floor(f32(w * f32(zoom_x)))`, `x0 = floor(x - ox*zoom_x)` as for any sprite - the
position does not depend on the cut). k is the index inside the source rectangle in drawing
order:

    a = max(0, -s0) ; b = min(w, bw - s0)          indices [a, b) of the rect lie in the bitmap
    mirror (x axis only): (a, b) = (w - b, w - a), texel(k) = s0 + w - 1 - k ; else texel(k) = s0 + k
    a >= b: nothing drawn
    neither axis stretched (dw == w AND dh == h):   column i shows texel(i) for a <= i < b
    otherwise (either axis stretched; both axes use this, also the one with dw == w):
        step  = (w << N) / dw                          (N = 20, integer division)
        first = a * dw / w        if a > 0 else 0      (integer division)
        end   = (b * dw - 1) / w  if b < w else dw     (integer division)
        for i in first .. end-1:
            k = a + (((i - first) * step) >> N)   if a > 0
            k = (i * step) >> N                   if a == 0
            column x0 + i shows texel(k) when k < b

These are the rules of "stretch_blt sampling" in the README for a source rectangle that
leaves the bitmap (left cut: the first column is floor(a*dw/w) and the walk restarts at the
first valid texel; right cut: one column fewer than the plain formula allows), applied to the
sprite, with mirror applied before the cut (a mirrored rect cut on its left behaves like an
unmirrored one cut on its right). Rows likewise with y, h, zoom_y (no mirror).

Measured columns (x0 = 10, zoom 2 unless noted; `texel u` per screen column):

| src_rect | zoom | mirror | first..last column | texel u per column |
|---|---|---|---|---|
| (-6,0,24,16) | 2x2 | no | 22..57 | 0 0 1 1 2 2 ... 17 17 |
| (6,0,24,16) | 2x2 | no | 10..44 | 6 6 7 7 ... 22 22 23 (texel 23 once: last column lost) |
| (-6,-4,36,24) | 2x2 | no | 22..68 | 0 0 1 1 ... 22 22 23 ; rows 18..48: 0 0 1 1 ... 14 14 15 |
| (20,0,12,8) | 2x2 | no | 10..16 | 20 20 21 21 22 22 23 |
| (-6,0,12,16) | 2x2 | no | 22..33 | 0 0 1 1 2 2 3 3 4 4 5 5 |
| (0,4,24,16) | 2x2 | no | rows 10..32 | v = 4 4 5 5 ... 14 14 15 |
| (6,0,24,16) | 1x2 | no | 10..26 | 6 7 8 ... 22 (17 columns: texel 23 lost although zoom_x = 1) |
| (0,4,24,16) | 2x1 | no | rows 10..20 | v = 4 5 6 ... 14 (row of texel 15 lost) |
| (6,0,24,16) | 1x1 | no | 10..27 | 6 7 ... 23 (nothing lost) |
| (-6,0,24,16) | 2x2 | yes, x0 = 30 | 30..64 | 17 17 16 16 ... 1 1 0 (35 columns) |
| (6,0,24,16) | 2x2 | yes, x0 = 30 | 42..77 | 23 23 22 22 ... 6 6 |
| (-6,-4,36,24) | 2x2 | yes, x0 = 30 | 42..88 | 23 23 22 22 ... 1 1 0 |
| (-6,0,12,16) | 2.5x3 | yes, x0 = 27 | 27..40 | 5 5 5 4 4 4 3 3 2 2 2 1 1 0 |

vkxp today: **differs** in most cut cases (it cuts the rect and restarts the stretch, and does
not drop the last column/row): e.g. zoom 2x2: (6,0,24,16) 32 pixels, (0,4,24,16) 48,
(-6,-4,36,24) 79; left/top cuts at zoom 2 are equal (0), at 1.5 and 2.5x3 they are not
(60..1900 pixels); zoom 1x1 is equal in all 8 cases; mirrored cuts differ.
What must change in `stretched`/`cut_span`: produce, per axis, the column range
`[first, end)` and the index walk above instead of "cut `dest` down to the pixels that show
what is left"; since the protocol wants the source rectangle inside the bitmap, the op must
carry the restart (source index `a`, step from the uncut `w`/`dw`) - i.e. the destination
`[x0 + first, x0 + end)` showing source texels `texel(a + (((i - first)*step) >> N))` with
`step = (w << N)/dw` of the UNCUT rect; when a == 0 this is the ordinary walk with the
destination shortened to `end`.

#### angle != 0

Probes `srcrect` frame 4 and `geom2` frame 0: 32 turned sprites (angle 30, zoom 2, with and
without mirror). Rule, 0 wrong pixels:

    a turned sprite is drawn only if its src_rect lies entirely inside the bitmap
    (sx >= 0, sy >= 0, sx + w <= bw, sy + h <= bh); otherwise NOTHING of it is drawn.

22 rects leaving the bitmap - by 6, by 4, by one texel on each side ((-1,0,24,16),
(1,0,24,16), (0,-1,24,16), (0,1,24,16), (0,0,25,16), (0,0,24,17)), entirely outside - draw
nothing; the 10 rects inside ((0,0,24,16), (4,3,10,8), (23,15,1,1), (1,1,22,14), (0,0,23,16),
(1,0,23,16), (0,1,24,15), mirrored ones) are drawn by the ordinary turned rule.

vkxp today: **differs**: draws the part of the rectangle that is inside the bitmap
(127..1701 differing pixels per case). What must change: `turned_placement` returns `None`
when `src_rect` is not contained in the bitmap's rectangle.

## Decisions

- Turned sprites are already rendered on the guest into cached bitmaps; the viewport rule
  and the bush rule change what that rendering computes, nothing else.
- An unrotated sprite that is stretched on either axis and whose source rectangle leaves its
  bitmap is drawn the same way, rendered by the rule of item 5 into a cached bitmap and placed
  unscaled, instead of a stretch quad of a cut rectangle. The protocol does not change. A
  source rectangle inside the bitmap, and an unstretched sprite, stay stretch quads as today.
- A zoomed unrotated sprite with a bush depth becomes two stretch quads, the upper and the
  lower part of item 3b, each with its own source rows and destination rows; when `sb` is 0
  only the upper one is drawn. With a vertical zoom of 1 this is what is drawn today.
- Measured only with opaque texels and opacity 255: the faint rows of a turned sprite, and
  item 3b. The opacity of faint rows is the opacity shifted right by one, as for unrotated
  sprites.
- The requirement "Sprite bush depth" of `core-scene` described the lower rows of the source
  rectangle; it is reworded to the screen rows the measurements show.

## Follow-ups

- A stretched sprite that both has a bush depth and a source rectangle leaving its bitmap was
  not measured. It is drawn as the model of the measurements (`predict_case`) draws it: the
  columns by the rule of item 5, the rows as the two stretches of item 3b with the rows outside
  the bitmap left out, not cut by the rule of item 5. To be measured.
- `Assembly::stretched` still restarts the stretch of a cut rectangle at its corner. Sprites no
  longer reach it with such a rectangle while stretched; planes, which are out of scope, do
  with tiles cut by their bitmap. Whether the original draws those by the rule of item 5 is
  not measured.
- The comment on `Sprite::bush_depth` in `element.rs`, outside the scope of this change, still
  speaks of the rows at the bottom of the source rectangle; it should speak of screen rows.
- The measured samples are committed without the topic `negzoom`, which belongs to item 1 of
  the report and not to this change.
