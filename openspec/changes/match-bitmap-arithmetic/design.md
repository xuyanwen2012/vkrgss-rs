# Design

## Context

- All rules were measured with probe scripts in the original runtime under Wine. The write-up
  and the samples are in this clone under `local/measurements/` (ignored by git): `README.md`,
  sections "Bitmap#blt and stretch_blt with opacity", "stretch_blt sampling" and "Tone,
  viewport tone and colour", with `bitmap-blt-samples.txt`, `bitmap-stretch-samples.txt` and
  `tone-samples.txt`. The numbers are measured facts about behaviour; a selection of each file
  is committed as test data. Read the README sections before starting: they hold the five
  exception pairs and the clipping rules in full.
- `vkxp-pixel` has the blend functions (the path for a destination alpha below 255 is marked
  as a placeholder), `stretch_index` and `apply_tone`, with provenance lists; `vkxp-core`'s
  bitmap arena calls them; `crates/vkxp-vk/shaders/pixel.glsl` mirrors the tone.
- `vkxp-core`'s `Color` and `Tone` clamp both bounds today.

## Decisions

- One blend function for a destination with alpha, following the measured formula with the
  exception list as a small table; for destination alpha 255 it must give the same result as
  the screen formula, which a test checks over the whole grid. Provenance: reverse-engineered,
  `oracle measurement 2026-10-09, RGSS103J, bitmap blt probe`.
- Stretch: fixed-point step with 20 fractional bits in 64-bit arithmetic; the clipping rules
  of the README implemented as written, each with its measured samples.
- Value types: lower bound of `Color` components removed (upper bound and `Tone`'s range
  stay as measured); conversion to a stored pixel as the requirement says. What reaches the
  protocol (sprite colour) keeps being clamped to 0 to 255 there.
- Tone: grey first with the measured weights, then the offsets; GLSL changed in step.
- Tests in other crates that pin old values are recomputed, not loosened.

## Risks / Trade-offs

- [The five exceptions look arbitrary] → They are measured; keep them as data with a comment
  that says so.

## Follow-ups

- `hue_change`; why exactly those five pairs.
- `stretch_blt` loses no column on the right when both rectangles have the same size. The
  measurement has that case only with both axes equal; the implementation treats the whole
  transfer as a block transfer then. Whether one axis of equal length next to a stretched one
  behaves the same is not measured.
- Clipping rules combined (a source rectangle leaving the source while the destination
  rectangle is clipped on the same side) follow from applying both rules; not measured.
- Text drawing keeps leaving a pixel alone where the coverage is 0, as the existing
  requirement says. Whether the original writes the font colour with alpha 0 onto a fully
  transparent pixel there, as `blt` does, is not measured.
- The compositors use the same normal blend for a frame cleared with an alpha below 255, so a
  transparent texel now leaves its colour (alpha 0) on a fully transparent frame pixel. The
  additive and subtractive blends onto a destination with alpha stay placeholders.
- Colour components far below zero are reduced modulo 256 after conversion to a 64-bit
  integer; the original's behaviour beyond the 32-bit range is not measured.
