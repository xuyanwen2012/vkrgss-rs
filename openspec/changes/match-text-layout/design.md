# Design

## Context

- Measurements: `local/measurements/README.md`, the sections whose headings start with
  "Text:", with `text-samples.txt` (4398 lines; its comment lines explain the columns),
  `text-model.py` (the rules as code) and the probe sources (all in this clone, ignored by
  git). Read the sections first; they hold the exact formulas this document names. Selections
  of the samples are committed as test data; the Python model is not committed.
- Measured with Noto Sans CJK SC and Liberation Sans under Wine's GDI and FreeType. The README
  marks what may be Wine's own: the decrement in the choice of pixels per em, and the `ceil`
  and the `size / th` factor of the squeeze steps.
- `vkxp-text`: `LoadedFace::pixels_per_em` and `LoadedFace::baseline` (`face.rs`) work with a
  fractional size, which is the cause of the one-pixel offset; `TextEngine::measure` returns
  the size as height and `(0, size)` for the empty string; `TextEngine::layout` rounds
  advances from the fractional size, has no overhang, gives control characters width 0 and
  takes missing characters from a fallback font; `TextEngine::load` synthesises no style.
- `vkxp-core`: `Bitmaps::draw_text` and `horizontal_layout` (`text.rs`) centre with
  `floor((h - size) / 2)`, squeeze by resampling the mask, and skip pixels whose alpha comes
  to 0. The rasteriser is reached through the `TextRasterizer` trait, implemented in
  `vkxp-binding` by an adapter over the text engine.
- A font is sized either by its own line box or, for a replacement of a known Windows family,
  by that family's line box (`line_box.rs`); both feed the same `box` of the sizing rule.
- Fonts used in tests are the two committed test fonts of `vkxp-text`; the measured samples
  are for other fonts, so tests of measured numbers load those fonts from the system when
  present (Noto Sans CJK SC static is not installed system-wide; Liberation Sans is) and the
  rules themselves are tested on the committed fonts with values computed from the font's
  tables.

## Decisions

- The text engine exposes what the drawing needs as data: text height, ascent, per-character
  advances, left overhang of the first and right overhang of the last glyph, the average
  character width, and a way to rasterise a run with given advances. `TextRasterizer` is
  extended accordingly; squeezing is then decided in `vkxp-core` by the measured step rule and
  rendered by asking for narrower advances, with glyphs condensed horizontally by the step.
- Per-character fallback stays for drawing (a character the font lacks is drawn from a
  fallback font, which is better than a box for play), but its advance is the selected font's
  missing-glyph advance, as measured. PLACEHOLDER: the original draws the missing glyph.
- Synthesised bold: embolden by one pixel and add one pixel to each advance and to the average
  width; synthesised italic: a shear of the glyph bitmaps with unchanged advances, which
  produces the overhangs.
- The whole-cell blend uses the existing `blt` arithmetic of `vkxp-pixel`.
- Argument errors are raised in the binding.

## Risks / Trade-offs

- [Rules that are Wine's rather than Windows'] → They are isolated in one function each and
  marked PLACEHOLDER with the README's wording, so a later measurement on Windows can replace
  them.
- [Existing tests pin heights equal to the size] → True only for fonts whose line box equals
  the em; recompute the others.

## Follow-ups

- Overhangs of squeezed and synthesised glyphs; glyph coverage levels; real Windows fonts.
