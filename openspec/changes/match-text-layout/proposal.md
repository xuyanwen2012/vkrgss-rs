# Proposal

## Why

With the same font file on both sides, text in vkxp has the same left edges and widths as in
the original in the test game's menu but sits one pixel lower in most lines, and a systematic
measurement of `text_size` and `draw_text` shows where the rules differ: the original sizes a
font at a whole number of pixels per em and takes the text height from the rounded ascent and
descent, so the height is not always the font size (22 gives 21, 23 gives 24 with the font
measured); it rounds advances at that whole size; it keeps room for a first glyph that
reaches left of the pen; it centres with `max(0, (h - th) div 2)`; it squeezes wide text by
setting it in a narrower font in steps of ten percent instead of scaling the run; it blends
the whole text cell; bold adds one pixel per character. vkxp differs in each of these: 4229 of
7304 measured widths and the heights at 37 of 91 sizes.

Everything was measured with Noto Sans CJK SC and Liberation Sans under Wine's GDI, not with
Windows fonts on Windows. The rules are GDI's way of working and are adopted as such; details
that may be Wine's own are marked.

## What Changes

- Font sizing: whole pixels per em chosen from the line box, text height from the rounded
  ascent and descent, advances rounded at that size, missing and control characters measured
  as the font's missing glyph.
- `text_size`: width with the first glyph's left overhang, the measured height, `(0, 0)` for
  the empty string.
- `draw_text`: the measured vertical rule, overhang-aware alignment, squeezing in the steps
  90, 80, 70 and 60 percent with whole-pixel advances, blending of the whole text cell.
- Bold without a bold face adds one pixel per character; italic without an italic face slants
  the glyphs and keeps the advances.
- `Font#size=` outside 6 to 96 raises `ArgumentError`, and `draw_text` and `text_size` with a
  non-string raise `TypeError`, as the original does.

Out of scope: glyph shapes and coverage values, real Windows fonts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: sizing, measurement, synthesised styles.
- `core-bitmap`: placement, squeezing and blending of drawn text.
- `rgss-bitmap`: argument errors.

## Impact

- Crates `vkxp-text`, `vkxp-core` (text drawing and the rasteriser interface), `vkxp-binding`
  (adapter, `Font`, `Bitmap`), and tests that pin text sizes.
