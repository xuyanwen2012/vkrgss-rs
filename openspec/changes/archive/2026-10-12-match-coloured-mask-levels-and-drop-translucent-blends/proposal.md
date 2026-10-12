# Match the level of coloured masks and drop the translucent blends

## Why

Two of the differences `docs/compatibility.md` lists can be closed with what is known now.

- **Transition masks.** The level of a coloured mask pixel is computed as
  `floor((2989 r + 5866 g + 1145 b) / 10000)`, which is exact for grey and off by one for
  27,957 of the 16.7 million colours. The original's rule was read from its code and
  confirmed on 1,473,427 colours measured on RGSS103J and RGSS104E without a mismatch:
  `(5014911 r + 9841699 g + 1920606 b) >> 24` in unsigned 32-bit arithmetic.
- **Blending onto a translucent bitmap.** `blend_add` and `blend_sub` have a second,
  textbook formula for a destination whose alpha is below 255. The measurement of
  2026-10-10 showed that the original has one formula, that its screen has no alpha, and
  that no script-visible operation blends additively or subtractively into a bitmap with
  alpha. The branch is never taken by a frame and is not the original's in any case.

The same measurement saw the threshold of a masked transition at 0, not -1, at step 1 of a
transition of 257 frames.

## What Changes

- The level of a mask pixel follows the original's integer rule.
- `blend_add` and `blend_sub` use their measured formula for every destination and keep the
  destination's alpha; `blend_add_translucent` and `blend_sub_translucent` go.
- The threshold of a masked transition step is not below 0.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: the mask level.
- `frame-composition`: one formula for the additive and the subtractive blend.
- `host-session`: the threshold's lower bound.

## Impact

`crates/vkxp-vk` (`shaders/present.frag`, `shaders/pixel.glsl`, tests and test data),
`crates/vkxp-pixel` (`arith.rs`, the provenance list), `crates/vkxp-compose-ref` (tests that
name the two functions), `crates/vkxp-host` (the threshold of `step_mix`),
`docs/compatibility.md`. Frames of a transition under a coloured mask change in pixels whose
mask colour is one of the 27,957; no other frame changes.
