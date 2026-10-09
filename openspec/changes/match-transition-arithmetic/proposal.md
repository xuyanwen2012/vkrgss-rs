# Proposal

## Why

Measured on the original runtime, the picture shown when `Graphics.transition(N)` returns
follows exact integer formulas, both for the plain cross-fade and for the mask weight of a
transition picture. vkxp's mix differs from it in almost every pixel by small amounts, treats
the mask differently (it stretches a small mask where the original tiles it, and uses another
threshold), and in a run with real mask files it reported for every mask that the bitmap does
not exist and fell back to a cross-fade.

## What Changes

- The presenter mixes with an integer weight: `(old * (256 - an) + new * an) >> 8` for the
  cross-fade, and the measured threshold formula for masks, on integer frame values.
- The session computes `an` and the mask threshold `T` for each step from the step number and
  the duration.
- Masks are tiled over the frame, and their value is the luminance of red, green and blue.
- The defect that makes the host report an existing mask as missing is found and fixed.

Out of scope: the 214 of 114199 measured mask colours whose luminance is one higher than the
formula used here (noted as a follow-up).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: integer transition mix and mask rule.
- `host-session`: step values and mask delivery.

## Impact

- Crates `vkxp-vk` (present pass), `vkxp-host` (session, headless display), and whichever guest
  crate turns out to cause the missing mask.
