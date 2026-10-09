# Design

## Context

- `vkxp-pixel` holds the arithmetic (`effective_alpha`, `blend_normal`, `blend_add`,
  `blend_sub`, `blend_color`, `apply_tone`) with a provenance list in which everything is a
  textbook placeholder; `vkxp-compose-ref` and the guest's bitmap operations call it, and
  `crates/vkxp-vk/shaders/pixel.glsl` mirrors it line for line. CPU and GPU must agree bit for
  bit.
- `blend_normal` takes a destination with alpha and serves both the frame (always opaque) and
  `Bitmap#blt` (any destination alpha).
- Measured on the original runtime (`RGSS103J.dll` under Wine) with a probe drawing 256 by 256
  grids over the background (88, 120, 200): the formulas of the spec reproduce every one of the
  measured pixels of five screens. A selection of the measurements is in this clone at
  `local/screen-blend-samples.txt` (ignored by git), one sample per line as described by its
  comment lines; the measured numbers are facts about the original's behaviour, not its code or
  assets, and a selection of them is committed as test data.

## Decisions

- Replace the formulas for an opaque destination and keep one code path per function: when the
  destination alpha is 255 the measured formula applies and the result's alpha is 255; for a
  destination alpha below 255 the existing placeholder computation stays (PLACEHOLDER, listed
  as such under its own name so the provenance list tells the two apart).
- `effective_alpha` becomes `(alpha * opacity + 127) / 255` everywhere.
- The colour and the tone offsets are replaced everywhere; the grey part of the tone keeps its
  placeholder.
- The GLSL is changed in step, with the existing CPU-against-GPU tests as the proof.
- Tests: the committed samples (`crates/vkxp-pixel/tests/data/`), plus the formula's own
  properties. Expected pixels in other crates' tests that were computed by hand with the old
  rounding are recomputed.
- Provenance source string: `oracle measurement 2026-10-09, RGSS103J, screen blend probe`.

## Risks / Trade-offs

- [Many tests pin old rounded values] → Expected; update them to the new arithmetic, do not
  loosen them into ranges.
