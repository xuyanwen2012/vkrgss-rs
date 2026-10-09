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
- Found while implementing; nothing was built for these:
  - A character drawn from a fallback font is set at the selected font's missing-glyph
    advance, as decided. Where that advance is narrower than the glyph (Chinese text in a
    Latin-only font: 0.75 em in Liberation Sans) neighbouring glyphs overlap. Is that wanted
    for play, or should the fallback glyph keep its own advance?
  - Ascent and descent are rounded each on its own, so a line box equal to the em does not
    always give a height equal to the size: the 220 + 36 of 256 box gives 33 at size 32 and
    97 at size 96 (82.5 and 13.5 both round up). The README states height = size for such
    fonts; which is right needs a measurement with such a font.
  - Synthesised styles follow what the samples show of GDI rather than the two decisions
    above as worded: bold emboldens the outline by a 24th of the em (the advance and the
    average width still grow by one pixel), italic slants the outline by 0.25, and the
    overhangs are taken from the box of the emboldened outline with its corners slanted.
    "One pixel" and a shear of the bitmaps do not give the measured left overhangs (a bold
    `A` reaches one pixel left of the pen). With this, width, height and advances agree in
    2022 of the 2024 width samples of all four styles and in all 736 committed ones (sizes
    12, 22, 48, 96), and the overhangs of the 73 regular and bold samples agree.
  - Still different: the right overhang of an italic glyph in 12 of 108 samples and of a bold
    italic one in 4 of 104, by one pixel (the box GDI slants comes from the hinted glyph;
    the bottom of that box is fitted here, constant `SHALLOW_BOTTOM`, and its top and right
    edge are not). These samples and the bold italic alignment samples, of which 37 of 426
    differ in an ink column, are not committed. The left overhang of `Y` at size 6 (regular
    and italic, 4 pixels per em) is 1 in the original and 0 here; size 6 is not among the
    committed sizes.
  - Squeeze samples: the sample of `HH` in Liberation Sans at size 13 listed the widths 18,
    14, 12 where the rule also gives 8. A sweep of every rectangle width on the original
    (probe and raw output in `local/measurements/text-squeeze-sweep-*`) shows the text 8
    wide and right-aligned for the rectangle widths 8 to 11, as the rule says, so the
    committed line is corrected and the sweep is committed as `P` samples; vkxp reproduces
    its 300 cells column for column. The sweep of `WW` at size 13 is not committed: the
    original draws the squeezed text one pixel wider at the 60 percent step than here (an
    overhang of the squeezed `W`), which is the open point named above.
  - The measured vertical, alignment and squeeze samples are checked through the font
    adapter, which is in `crates/vkxp/src/font.rs` (the binary crate), not in
    `vkxp-binding`; for that `vkxp-core` makes the squeeze rule public
    (`TextMetrics::squeezed`). Samples of the font that is not committed are skipped without
    it. The last ink row (the baseline) and the first ink column are compared exactly; the
    first ink row and the last ink column within one pixel, since they depend on how the
    glyph is rasterised (no hinting here).
  - `Font#size=` keeps the whole part of a fractional size (`96.5` reads back as 96, in the
    original as 96.5), since the core font size is an integer. `Font.new(name, size)` and
    `Font.default_size=` are not range-checked; they were not measured.
  - The `TypeError` for text that is not a string has the wording of the embedded Ruby 1.8.7
    (`can't convert Fixnum into String`), the original's 1.8.1 says `cannot convert`.
  - A control character the font lacks now draws the font's missing-glyph box, as in the
    original; scripts that pass `"\n"` to `draw_text` show a box.
