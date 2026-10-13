## Context

- Bitmap operations live in `crates/vkxp-pixel` (arithmetic) and `crates/vkxp-core`
  (`bitmap.rs`, `text.rs`), bound in `crates/vkxp-binding/src/bitmap.rs` and `font.rs`.
  The binding defines classes by the runtime of the run.
- The measurement: `../../ref/tools/oracle/measurements/vx-v1-bitmap-REPORT.md` (seen from
  a clone under `wt/`), the models `vx_v1_bitmap_model.py` and `vx_v1_bitmap_xp.py`, the
  checks `vx-v1-bitmap-*-check.py` and the samples `vx-v1-bitmap-samples.txt` and
  `vx-v1-bitmap-*-samples.bin.gz`. The report is the authority for every rule below; where
  this text is shorter than the report, the report decides. `ref/RGSS202E_api.md` and
  `vx-v1-api-REPORT.md` (sections 3.3 and 3.4) hold counts, errors and defaults.

### Measured rules (RGSS2, both DLL versions identical; integers, `/` truncates toward zero, straight alpha)

1. `blur`: nothing when the width or the height is below 3. Otherwise one pass of a 3 by 3
   box read from a copy, the edge pixel repeated beyond the edges; each of r, g, b, a is
   the sum of nine divided by 9, a transparent neighbour giving the colour it holds.
   (102 cases, 27 168 pixels.)
2. `radial_blur(angle, division)`: both arguments as integers, `angle` clamped to 0 to
   360, `division` to 2 to 100; nothing when the width or the height is below 3. In
   doubles: `step = angle * 6.28318 / 360.0 / (division - 1)`,
   `half = (division - 1) * step * 0.5`, `a_i = step * i - half` for i from 0 to
   division - 1. Centre `cx = w >> 1`, `cy = h >> 1`; `dx = x - cx`, `dy = y - cy`; sample
   `sx = trunc(cos(a_i) * dx + cx - sin(a_i) * dy + 0.5)`,
   `sy = trunc(sin(a_i) * dx + cy + cos(a_i) * dy + 0.5)`; mirrored at the edges until
   inside (`v < 0` becomes `-v`, `v >= n` becomes `2n - 1 - v`); nearest pixel of a copy;
   each channel is the sum divided by `division`. (468 cases, 530 858 pixels.)
3. `gradient_fill_rect`: colour components truncated and taken modulo 256; `n` is the
   width, or the height when vertical; nothing when `n < 2`. Column or row i gets
   `(c1 + (i * (c2 - c1)) / (n - 1)) & 255` per channel, alpha included, in signed
   truncating division. It overwrites and clips as `fill_rect`. With seven arguments the
   original takes the gradient as vertical whatever the seventh is, also for `false` and
   `nil`; with a `Rect` the argument works as documented. (300 cases, 80 472 pixels.)
4. `clear_rect`: `fill_rect` with (0, 0, 0, 0), clipped; a width or height of 0 or less
   does nothing.
5. `blt`, `stretch_blt`, `fill_rect`, `set_pixel`, `get_pixel`, `hue_change` and the
   decoding of pictures give the same bytes in RGSS2 as in RGSS1 (two originals compared).
   `Bitmap.new(path)` for a file that is empty or cannot be decoded raises
   `RGSSError: failed to create bitmap` (RGSS1: a transparent bitmap of 32 by 32).
   `Bitmap.new(w, h)` raises the same error above 4 194 304 pixels.
6. Text. Font defaults: name `["Verdana", "Arial", "Courier New"]`, size 20, bold and
   italic false, shadow true, colour white. Shadow: first every pixel of the text cell is
   blended at (+1, +1) with the source `(0, 0, 0, (cov * colour_alpha + 127) / 255)` by the
   arithmetic of `blt`, cov being the coverage of the glyphs; then the text is blended as
   in RGSS1. The shadow is clipped by the bitmap only and may lie one pixel outside the
   rectangle given; a colour alpha of 0 draws nothing. Alignment, vertical placement,
   clipping, blending and `text_size` (which does not count the shadow) are those of
   RGSS1. Squeeze: `r = 1.0; while width > w and r > 0.6: r -= 0.1` in doubles, so that
   rounding gives five steps (90, 80, 70, 60, 50 percent) where RGSS1 has four; at the
   last the text is drawn from the left edge and cut off. A text argument that is not a
   String is drawn as its `to_s`.
7. `dup` and `clone` copy the pixels and the font values, the shadow included. `font=`
   copies name, size, bold, italic and colour and not the shadow.

Not Windows-exact and outside the standard as for XP: the glyph pixels themselves.

## Goals / Non-Goals

**Goals:** the operations and rules above in a game of RGSS2, with the bytes of the
original. XP unchanged.

**Non-Goals:** the six places where the report found vkxp's rules of XP to differ from
both originals (section 10 of the report: opacity above 255, clip order of a mirrored
transfer, a disposed source, a 32-bit BMP with zero alpha, pixels touched without ink,
the lower limit of `Color`): a change of their own; the pixels that `draw_text` touches
without ink (the work area of section 6e); the loose storage of `Font.default_*` objects;
`NKF`.

## Decisions

- The arithmetic of rules 1 to 4 goes into `vkxp-pixel` with entries in its provenance
  list as measured, and `vkxp-core` applies it to bitmaps; the binding adds the four
  methods in RGSS2 only, with the counts and errors of `vx-v1-api-REPORT.md` section 3.3.
  Where the original ends with a page fault (no argument to `clear_rect` or
  `gradient_fill_rect`) vkxp raises `ArgumentError`.
- `radial_blur` uses `f64` with the constants as written (6.28318, not a better pi) and
  the `sin` and `cos` of the Rust standard library. The report found no pixel that depends
  on the last bit of them in 530 858; the code says that this is where a difference would
  come from.
- The seven-argument form of `gradient_fill_rect` is vertical always, as measured, with a
  comment that names it as the original's behaviour.
- The font of RGSS2 gets `shadow`; the font defaults follow the runtime. `Font#shadow`
  and `Font.default_shadow` exist in RGSS2 only; the reader of `Font#shadow` gives true or
  false. The text routine draws the shadow layer from the same coverage it draws the text
  from.
- The squeeze of RGSS2 is the loop as written, in `f64`; RGSS1 keeps its own rule.
- The font names "Verdana" and "Arial" resolve as the requirement "Default substitutions"
  of `text-rendering` decides today; nothing about substitutes changes here.
- Tests: for rules 1 to 4, committed samples taken from the report's sample files (inputs
  made by a formula, outputs of the original), enough to cover every branch named above;
  for the shadow and the squeeze, tests on the coverage of a test font that check the
  shadow layer and the chosen step, not glyph pixels of Windows.
- The names this change adds leave the pending list of the method test; the method list
  of RGSS2 gains `Bitmap` and `Font`.

## Edits outside the named files

Each is a few lines that a requirement needs.

- `crates/vkxp-core/src/lib.rs`: the export of `Squeeze`, which the binding passes to the
  text routine to choose the squeeze of the runtime.
- `crates/vkxp-binding/src/guest.rs`: `Bitmap` and `Font` are defined with the runtime of
  the run, and the guest starts with the font defaults of that runtime (`font_names` of the
  configuration is what RGSS1 starts with).
- `crates/vkxp-binding/tests/cases/mod.rs`: two cases for the scenarios "XP" (no `blur`,
  no `shadow`, the default size 22). The scenarios of RGSS2 are in a test binary of their
  own, `tests/bitmap_rgss2.rs`.

## Follow-ups

- The provenance entries of rules 1 to 4 are a list of their own in `vkxp-pixel`
  (`BITMAP_RGSS2_PROVENANCE`), beside the list of the bitmap arithmetic of XP, whose test
  names its two entries; that test and its expected value stay as they were.
- The message panel of vkxp (`crates/vkxp-binding/src/panel.rs`) takes its font from the
  font defaults of the game, so in a game of RGSS2 its text has size 20 and a shadow. Not
  measured against anything; whether the panel should have a font of its own is open.
- `Font.default_name` of RGSS2 is the list of the English DLL whatever `Library=` says;
  the Japanese DLLs were not measured (open point 2 of the report).
- The errors of a wrong count in the four new methods carry the message of the original
  (`wrong # of arguments (4 for 2)`); the older methods of `Bitmap` keep Ruby's message in
  RGSS2 as in RGSS1.
