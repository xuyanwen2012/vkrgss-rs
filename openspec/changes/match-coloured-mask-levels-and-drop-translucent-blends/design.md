# Design

## Context

- `crates/vkxp-vk/shaders/present.frag` computes the mask level, with a `PLACEHOLDER`
  comment; the tests in `src/tests/presentation.rs` compare it with the `lum` lines of
  `src/tests/data/transition-mask-samples.txt`, a selection that the old formula
  reproduces. There is no copy of the formula in Rust outside the tests.
- `crates/vkxp-pixel/src/arith.rs` has `blend_add` and `blend_sub`, which call
  `blend_add_translucent` and `blend_sub_translucent` (provenance `Textbook`) when the
  destination's alpha is below 255. The shader library `shaders/pixel.glsl` mirrors the
  arithmetic. `arith.rs`, `bitmap.rs` and `vkxp-compose-ref/tests/scenarios.rs` have tests
  that name the two functions as the placeholders that are left.
- `transition-mask-colour-samples.txt` in this change's directory holds 382 measured
  `lum r g b -> L` lines: 120 colours where the old formula is one low, 120 where it is one
  high, the rest where it is right.
- The session computes the threshold of a masked step as `256 * step / duration - 1` for
  step 1 and later (`step_mix`).

## Decisions

### Mask level

`L = (5014911 * r + 9841699 * g + 1920606 * b) >> 24`, with `uint` operands in the shader:
the sum reaches `255 * 2^24` and does not fit a signed integer. Nothing is added before the
shift, and alpha is not used. The weights sum to `2^24`, so a grey pixel keeps its value.
The `PLACEHOLDER` comment becomes a provenance note: read from the code of RGSS104E and
measured on RGSS103J and RGSS104E, 2026-10-12, PNG masks of 24 and 32 bits.

The samples file moves to `crates/vkxp-vk/src/tests/data/` beside the existing one, and the
test that reads the `lum` lines of the old file reads both. The comment at the head of the
old file that says only colours the old formula reproduces were selected is corrected.

### One formula for the additive and the subtractive blend

`blend_add` and `blend_sub` lose their branch: for every destination,
`c = min(255, dst.c + ((src.c * (a + 1)) >> 8))` and `c = max(0, dst.c - ...)` with
`a = effective_alpha(src.a, opacity)`, the destination unchanged for `a = 0`, the
destination's alpha kept. The two `_translucent` functions, their entries in the provenance
list, and the same branch in `pixel.glsl` if it has one, are removed. The entries of
`blend_add` and `blend_sub` say: measured on opaque destinations; the original's screen has
no other. Tests that named the two functions as placeholders are changed to say that no
blend function is a placeholder; tests of the removed functions go.

`blend_normal_translucent` is another function, measured, and stays.

### Threshold

The threshold of a masked step is `max(0, 256 * step / duration - 1)`. The formula gives -1
only at step 1 of a transition longer than 256 frames; one such case was measured.

## Risks

- [A caller blends into a bitmap with alpha after all] → the result is then the measured
  share with the alpha kept, which is what the original's one formula would give.

## Follow-ups

- Mask files with a palette, in grey of 8 or 16 bits, and JPEG masks take other conversion
  routines in the original; they were not measured.
