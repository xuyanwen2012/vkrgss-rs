## Context

- The window of RGSS1 is in `crates/vkxp-core/src/window.rs`: it builds quads in the
  window blend mode from the skin (requirements "Window ... as in the original" of
  `core-scene`, "Window blend arithmetic of the original" of `frame-composition`, "Bilinear
  window stretch of the original", "Stretch sampling of quads"). The scene classes are
  defined by runtime in the binding since `move-elements-between-viewports-in-rgss2`.
- The measurement: `../../ref/tools/oracle/measurements/vx-v1-window-REPORT.md` (seen from
  a clone under `wt/`), its Python model `vx-v1-window-model.py`, the case files
  `vx-v1-window-g*-cases.py` and the samples `vx-v1-window-{facts,anim,openness}-samples.txt`.
  The report is the authority for every rule below; where this text is shorter than the
  report, the report decides. `wblend` is the window formula and `sblend` the normal
  formula of `frame-composition`; `stretch_index` is the nearest sampling of "Stretch
  sampling of quads"; all four, the bilinear window stretch and the arithmetic of
  `Bitmap#blt` were found unchanged from RGSS1.

### Measured rules (RGSS2, both DLL versions identical)

Skin, fixed coordinates in a skin of 128 by 128; outside the skin is transparent:
background (0,0,64,64); pattern (0,64,64,64); frame corners 16 by 16 at (64,0), (112,0),
(64,48), (112,48); frame edges top (80,0,32,16), bottom (80,48,32,16), left (64,16,16,32),
right (112,16,16,32); arrows up (88,16,16,8), down (88,40,16,8), left (80,24,8,16), right
(104,24,8,16); cursor (64,64,32,32) with a border of 2; pause images 16 by 16 at (96,64),
(112,64), (96,80), (112,80). (64,96,64,32) is not read.

1. Background: a bitmap of (W-4) by (H-4) drawn at (2,2). The 64 by 64 source is stretched
   bilinearly with numerator 63 when both sides exceed 32 and the size is not 64 by 64,
   else by `stretch_index(i, 64, d)`. Pattern: tiled from the top left of that bitmap;
   where the alpha of the background and of the pattern are above 0, colour =
   `wblend(background, pattern, pattern alpha)`, alpha stays the background's; the last
   column uses `sblend` when W-4 is odd. Alpha on the screen:
   `(a * ((back_opacity * opacity) / 255) + 127) / 255`.
2. Frame: a bitmap of W by H composed with the arithmetic of `blt` in the order top,
   bottom, left, right edge, then the corners top left, top right, bottom left, bottom
   right; edges tile in pieces of 32 from 16 to size - 16. Alpha on the screen:
   `(a * opacity + 127) / 255`.
3. On the screen every part is blended with `wblend`, the last visible column of a part
   with `sblend` when its count of visible columns is odd; the background counts one more,
   transparent, column. With openness below 255 every pixel uses `sblend`.
4. Openness, in double precision with `s = openness / 255.0`: `n = trunc(H * s)` rows are
   shown from row `top = trunc(H * (1 - s) / 2)`; screen row `top + i` shows window row
   `stretch_index(i, H, n)`. Contents, cursor, arrows and pause sign are drawn only at 255.
   Integer forms of the two expressions are wrong for some values.
5. Contents: pixel (u, v) at (16 + u - ox, 16 + v - oy), clipped to the inner rectangle
   (16, 16, W-32, H-32); alpha `(a * contents_opacity + 127) / 255`; over the cursor; shown
   without a skin as well.
6. Cursor: the pieces of RGSS1 (centre 28 by 28 stretched, bilinear with numerator 27 when
   both sides exceed 14; edges stretched; corners 2 by 2) at (16 + x, 16 + y), not moved by
   ox and oy, clipped by the window's rectangle. Opacity
   `max(0, contents_opacity - 8 * m)` with `m = c` for `c <= 20`, else `40 - c`;
   `c = (c + 1) mod 40` per `update` of an active window; `active = false` sets `c = 16`
   at once; `c` counts also while the window is hidden, closed or without a skin.
7. Pause sign: image `(p / 16) mod 4` at (W/2 - 8, H - 16) with alpha `min(255, 32 * p)`;
   `p = 0` when `pause` turns true, +1 per `update` while pausing. No opacity applies.
8. Arrows, drawn in the order down, left, right, up, no opacity: down when
   `ch - oy > H - 32` at (W/2 - 8, H - 12); left when `ox > 0` at (4, H/2 - 8); right when
   `cw - ox > W - 32` at (W - 12, H/2 - 8); up when `oy > 0` at (W/2 - 8, 4). The contents
   of 1 by 1 of a new window count as contents.
9. Latched state. A "refresh" happens in `update`, always in the writers of `width`,
   `height`, `ox`, `oy`, `contents_opacity`, `openness`, and in the writers of `x`, `y`,
   `active`, `pause`, `windowskin`, `contents`, `cursor_rect` when the value changes; not
   in `visible`, `z`, `opacity`, `back_opacity`, `viewport` or `Graphics.update`. A refresh
   sets "contents shown" to `visible`; with contents that are not nil it also decides the
   arrows anew and binds them to the skin of that moment. The background and frame bitmaps
   are built at `windowskin=` with another object and at every `width=` and `height=`; the
   cursor bitmap at a change of the skin or of the cursor's size. They are not rebuilt
   when the pixels of the skin change later; the pause sign and the arrows read the skin
   as it is.
10. A window is one element at its z; in a viewport it is moved by the rectangle less ox
    and oy, clipped, and ordered by its z among the viewport's elements. W or H of 0 or
    less draws nothing. Nothing needs `Window#update` to be drawn.

Open in the measurement: a skin narrower or lower than 64 (the original draws fewer
background columns and rows than the model; not fitted); pixels with a disposed skin or
contents; sizes above 600 and ox, oy beyond about 1000.

## Goals / Non-Goals

**Goals:** a window of a game of RGSS2 is drawn pixel for pixel as the original draws it,
for every case the model of the report covers. XP unchanged.

**Non-Goals:** the order of elements of equal z (the y key; it comes with the measurement
of sprites); the tone and colour of a viewport over a window (the same); skins smaller
than 64; text.

## Decisions

- `vkxp-core` gets the window of RGSS2 beside that of RGSS1, chosen when the window is
  created from the runtime the binding passes; nothing of the RGSS1 window changes.
- The window keeps the bitmaps the original keeps (background, frame, cursor), built at
  the calls of rule 9 from the skin as it is then, as bitmaps of the arena that only the
  window refers to. Extraction draws them with the ops that exist: quads in the window
  blend mode, the normal mode while openness is below 255, the row sampling of openness
  as a quad stretched vertically by the nearest rule. If a rule cannot be said with the
  existing ops, the smallest addition to the protocol and both compositors is made and
  recorded here with the case that needed it; the protocol version then goes up by one.
- The latch of rule 9 is state of the window in `vkxp-core`, driven by the calls the
  binding makes; the binding does not imitate it.
- `trunc(H * s)` and `trunc(H * (1 - s) / 2)` are computed in `f64` exactly as written.
  This is arithmetic in the guest; no float crosses the protocol.
- Tests: the cases of the report are rebuilt as fixtures from synthetic skins made by a
  formula in the test (no RTP picture). The expected pixels are committed as digests or
  small pictures produced by the report's model, for at least one case per rule and for
  thirty scenes of the random generator; the openness table and the blink and pause
  sequences are committed as numbers. A skin smaller than 64 follows the model and is
  marked as not measured.
- `openness` leaves the pending list of the method test.
