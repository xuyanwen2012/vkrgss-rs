## Context

`measure-text-as-gdi-does-in-rgss3` (archived design: decision on glyph images) draws an RGSS3
glyph by replacing the points of swash's outline with the hinted points that skrifa gives
(`hinted.rs`, `points`, `draw`, `to_26_6`). The symptom (ink on the lowest rows only, the
upper part missing) suggests a mismatch of units or of the y axis between the two point sets
(26.6 fixed point against pixels, font units against pixels, y up against y down), or a
scale applied twice, for the glyphs that the auto-hinter hints (VL Gothic); whether the
interpreter and CFF paths are affected as well is to be found.

Reproduce without the run-time package: a test font of the repository on the auto-hinter
path (the change used Noto Sans Symbols 2 and patched copies of Liberation Sans with a short
`prep` to select the auto-hinter) shows it if the cause is in the shared path. The after-merge
check on VL Gothic is done outside the flow (`VKXP_RGSS3_FONTS` and the test game).

## Goals / Non-Goals

Goal: whole, correctly placed glyph images under the metric rule of RGSS3, with the advances
and boxes of the archived change unchanged (its tests keep passing with their expected
values). Non-goal: glyph shapes equal to the original's.

## Decisions

- The fix is in the drawing of the hinted outline, not in the metrics.
- If the points of swash's outline and skrifa's hinted points cannot be matched safely for a
  glyph (count or order differs), that glyph is drawn from skrifa's hinted outline through
  its own pen into the rasteriser, or else plain; never partly.

## Tests and the cause

The tests of the three scenarios are `engine::tests::auto_hinted_glyphs_are_whole` and
`engine::tests::interpreted_and_cff_hinted_glyphs_are_whole` (unit tests, since the hinted box
and the glyph images are internal) and `a_hinted_line_of_text_is_whole` in
`tests/rgss3_metrics.rs`. The auto-hinter path is a copy of Liberation Sans without `fpgm` and
`prep`, the interpreter path Liberation Sans, the CFF path Source Code Pro 2.042, added
unmodified under `tests/fonts-cff/` (OFL 1.1) because no committed test font has CFF outlines.
A font of its own directory keeps the tests that load `tests/fonts` unchanged. Outside the
scope of the change, `THIRD-PARTY.md` got the few lines that list the new font beside the
other two fonts of the tests.

Before the fix all three failed, on every engine: `H` of Liberation Sans at 12 pixels per em
had its ink on rows 0..=-8 against the plain 9..=1, the same for both hinting engines, and `H`
of Source Code Pro rows 0..=-7 against 8..=1; the line `Mio attacks!` held 4 rows of ink
against 17 of the plain rule, as the frames of the test game showed.

The images themselves were whole; their placement was wrong. `hinted_glyph` rendered the
outline with `zeno::Mask::new(..).origin(Origin::BottomLeft).render()`. zeno 0.3.3 computes
the top of a bottom-left placement as the bottom of the bounds plus the mask's stored height,
which is 0 until the size has been computed by `inspect` (or set by `size`), so the top it
gave was the glyph's bottom row and every glyph was drawn its own height too low. `swash`'s
`Render`, which the plain rule uses, calls `inspect` before it renders, so plain glyphs were
placed right. The points of the hinted outline were in the right units and axis.
