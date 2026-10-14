## Why

After the change `measure-text-as-gdi-does-in-rgss3` was merged, the widths of the text of a
game of VX Ace equal the original's (the measured table of 28392 values passes with the fonts
of the run-time package, and the lines of the battle log of the test game end where the
original's end), but the glyph images are broken: in frames of that game drawn by vkxp each
character of VL Gothic shows ink only on its lowest three or four rows (the menu's
"Item", "Equipment" and the battle log are unreadable). Measured on 2026-10-14 against the
original's captures of the oracle run: the original's text of the battle log covers rows
17..30, 41..58, 65..82; vkxp's covers rows 31..34, 55..58, 79..82. The change's own tests
checked advances and boxes and only that a hinted glyph differs from a plain one, so nothing
caught it. Glyph shapes are not compared with the original, but a glyph must be whole and
in place.

## What Changes

- Glyph images of the metric rule of RGSS3 are drawn whole from the hinted outline: the ink
  of a hinted glyph covers the rows its hinted box gives, within a pixel of the plain glyph's
  rows, for every engine (auto-hinter, TrueType interpreter, CFF).
- Tests that would have caught it: on the test fonts of each engine, the rows and columns of
  ink of hinted glyphs against their hinted boxes and against the plain glyphs.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `text-rendering`: hinted glyph images are whole.

## Impact

`crates/vkxp-text` (`hinted.rs`, `glyph_cache.rs`, `engine.rs` as needed, tests). Nothing of
RGSS1 or RGSS2 changes.
