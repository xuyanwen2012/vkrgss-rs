# Proposal

## Why

`Graphics.transition` with a picture (the wipes, blinds and dissolves of the RTP's
`Graphics/Transitions`) is shown as a plain cross-fade. Battles and scene changes of most games
use these pictures.

## What Changes

- The presenter's mix of the held and the newest frame can be driven per pixel by a mask
  bitmap: where the mask is dark the new frame appears first, with a soft edge whose width is
  the transition's vagueness.
- The session passes a transition's mask and vagueness to the presenter instead of reporting
  that masks are not implemented.
- The mask is stretched to the frame when its size differs.

Out of scope: matching the original's exact threshold arithmetic (PLACEHOLDER until measured
with the oracle harness).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: masked mix.
- `host-session`: masked transitions.

## Impact

- Crates `vkxp-vk` (presentation shader and its inputs) and `vkxp-host` (session).
