# Proposal

## Why

Four more guesses about sprites, marked `PLACEHOLDER` or found beside them, were measured on
the original runtime, and vkxp differs in all four: a turned sprite in a viewport whose
origin is not zero (the original computes the box from the shifted position but the picture
from the unshifted one, so the picture is displaced inside its box and can vanish); the bush
depth of a turned sprite (the lowest rows of the bounding box, not of the source); the bush
depth of an unrotated sprite with a vertical zoom (two separate stretches; vkxp differs in
31 of 32 cases, and zoomed character sprites standing in bushes are common); and a stretched
sprite whose source rectangle leaves its bitmap (the columns of a cut `stretch_blt`; a turned
sprite with such a rectangle is not drawn at all).

## What Changes

- Turned sprites follow the measured rule inside viewports and for the bush depth, and are
  not drawn when their source rectangle leaves the bitmap.
- Unrotated zoomed sprites with a bush depth are drawn as two stretches.
- Unrotated stretched sprites with a source rectangle that leaves the bitmap show the
  measured columns and rows.

Out of scope: the protocol, the compositors and the shader (nothing there changes); other
alphas and opacities on faint rows than the ones measured; planes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: bush depth, rotated sprites, unrotated stretched sprites.

## Impact

- `crates/vkxp-core` (`turned.rs`, `extract.rs` outside `sort`, their tests and test data).
