## Context

- A sprite is extracted into quad ops by `crates/vkxp-core` (requirements "Sprite
  placement", "Unrotated sprites are stretched as in the original", "Stretched sprites
  with a cut source rectangle", "Sprite bush depth", "Bush depth and flash details of the
  original", "Bush rows of zoomed and turned sprites", "Rotated sprites as in the
  original" of `core-scene`). The scene knows the rules of its runtime since
  `order-elements-and-read-input-as-rgss2`.
- The measurement: `../../ref/tools/oracle/measurements/vx-v1-sprite-REPORT.md` sections 1
  and 2 (seen from a clone under `wt/`), its model `vx-v1-sprite-model.py`, the topics
  `vx-v1-sprite-topics.py`, the frames `vx-v1-sprite-frames/` and
  `vx-v1-sprite-wave-samples.txt`. The report is the authority for every rule below.

### Measured rules (RGSS2, both DLL versions identical)

Found equal to RGSS1 on the frames of VX: the blit, opacity, blend types, tone, colour, the
stretch walk and its cut rule, rotation, the sprite's flash, the bush row geometry up to
the height.

1. Place and size of an unrotated sprite, in doubles, in the coordinates of its viewport:
   `x0 = trunc(x - ox * zoom_x)` toward zero, `dw = floor(x0 + w * zoom_x) - x0`, likewise
   y; nothing when a zoom, w, h, dw or dh is 0 or less; the viewport's offset is added
   afterwards. (`90 * 0.7`: 62 wide at x 0, 63 at x 272.)
2. Unstretched and mirrored with a source rectangle that leaves the bitmap: the columns
   inside the bitmap are found without the mirror, `[a, b)`, and column i of them shows
   texel `sx + (a + b - 1 - i)`.
3. Bush rows: `op = opacity * bush_opacity / 255` truncated, `bush_opacity` not limited;
   the rows are not drawn when op is below 0 or above 255; alpha
   `(texel_alpha * op + 127) / 255`. They get no tone, no colour, no flash colour, and on
   an unrotated sprite no mirror. With `bd = min(bush_depth, dh)` and
   `sb = bush_depth * h / dh`: rows `[0, dh - bd)` show the source rows `[0, h - sb)`
   stretched; rows `[dh - bd, dh)` show `[h - sb, h)` stretched into `bush_depth` rows
   that end at the bottom, by the walk with the cut rule against the bitmap.
4. `bitmap=` sets `src_rect` to the whole bitmap only for another object; the same object
   and `nil` keep it.
5. Wave. `Sprite#update` adds `wave_speed / wave_length` (doubles) to `wave_phase` modulo
   360, also at amplitude 0 and when invisible. Drawing an unrotated sprite with
   `wave_amp > 0`: screen rows are grouped by `Y div 8`; `r0` is the first drawn row of the
   band counted from the sprite's top (the sprite's top or the clip's top), `k = j - r0`;
   `t = wave_phase + r0 * (360.0 / wave_length)`, then k times `t += 1.0 / wave_length`;
   `dx = round(wave_amp * SIN[floor(t) mod 360])` over whole degrees, an exact half
   toward zero; the row is drawn moved by dx. Texel rows are `sy + (j * h) div dh`;
   columns by the ordinary walk; the lowest `min(bush_depth, dh)` rows are bush rows and
   the rows are not split in two stretches. A negative amplitude: no wave, the sprite is
   squeezed into `dw + 2 * wave_amp` columns from `x0 - wave_amp`.
6. `wave_amp`, `wave_length`, `wave_speed`, `bush_opacity` are integers kept as written;
   `wave_phase` is a Float kept modulo 360 in [0, 360). Defaults 0, 180, 360, 0.0, 128.

Open in the measurement: a wave on a turned sprite; a wave on a sprite stretched in x
whose source rectangle leaves the bitmap; a negative `wave_length`; a `wave_length` of 0
(`Sprite#update` does not return in the original).

## Goals / Non-Goals

**Goals:** rules 1 to 6 in a game of RGSS2, pixel for pixel on the cases the model covers.
XP unchanged.

**Non-Goals:** the open cases, which are drawn as the report's model draws them (a turned
sprite without its wave) and marked as not measured; whether XP shares rule 3 for tone,
colour and mirror (not measured on XP).

## Decisions

- The extraction of a sprite branches on the rules of the scene's runtime; the functions
  of RGSS1 stay untouched and the functions of RGSS2 stand beside them.
- A waving sprite is extracted as one quad per run of rows that share dx and are
  contiguous in the source, with the ops that exist. If the measured texel rows cannot be
  said with a quad's stretch, the smallest addition to the protocol and both compositors
  is made and recorded here; the protocol version then goes up by one.
- `SIN` is a table of 360 values computed as `(d as f64).to_radians().sin()` narrowed to
  `f32`, as the report models it; the rounding rule is written once and tested on the
  measured rows.
- `wave_length = 0`: `Sprite#update` leaves the phase and the sprite is drawn without a
  wave. The original hangs there; vkxp does not imitate it.
- The new properties are defined in RGSS2 only and leave the pending list of the method
  test.
- Tests: the cases of the topics `clip3`, `size2`, `bushbig`, `bushop`, `wave1` to
  `wave6` and twenty scenes of the held-out generator, rebuilt by formula with expected
  pictures or digests from the report's model; the wave rows of
  `vx-v1-sprite-wave-samples.txt` as numbers.

## Notes of the implementation

- `order-elements-and-read-input-as-rgss2` was not merged when this change was implemented,
  so the scene had no value for the rules of its runtime. This change adds it under the
  names that change uses: `Rules { Rgss1, Rgss2 }`, `Scene::with_rules(rules, size)` and
  `Scene::rules()`, kept by `Scene::clear`. A sprite takes the rules of its scene when it is
  created.
- The sprite of RGSS2 is `crates/vkxp-core/src/extract/rgss2.rs`. It is drawn with stretch
  quads where they show exactly the measured texels, and from a picture of its own (the
  bitmap `turned.rs` keeps for the cut sprite of RGSS1) where they do not: a stretch cut by
  the bitmap's edge. The protocol and the compositors are unchanged.
- The cases are committed as text scenes with the digests of the model's pictures
  (`crates/vkxp-core/tests/data/sprite-rgss2-*.txt`, read by
  `vkxp_core::test_support::sprite_cases`), as the windows of RGSS2 are. The planes of
  `size2` and their viewports are left out of its scenes, because the size of a plane's tile
  belongs to the other change; the sprites of those frames are all there.
