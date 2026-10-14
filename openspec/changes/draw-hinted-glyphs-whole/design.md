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
