# Proposal

## Why

Windows are the last drawn element whose pixels were guessed. They have now been measured on
the original runtime: a reference model of the rules re-renders 372 captured frames (about 900
windows, 310 animation frames) with no differing pixel. vkxp matches the original in the
placement of contents, clipping, layer order and the blink of an active cursor, and differs in
background sampling, the tiling of the frame's edges, the blend arithmetic, the cursor's
stretch and opacity, and the pause sign.

## What Changes

- **BREAKING** (protocol version 4): a blend mode for window layers,
  `(s * (a + 1) + d * (256 - a)) >> 8`, with the measured rule for the last pixel of a row
  whose visible part has an odd number of pixels.
- Background: stretched by the stretch rule with the measured choice of numerator, or tiled
  from the inset corner; alpha from skin alpha, back opacity and opacity as measured.
- Frame: edges tiled under the corners with the measured phase and order; composed into its
  own bitmap first.
- Cursor: measured pieces, position, clipping, and opacity `contents_opacity - 8 * m`,
  including the inactive value.
- Pause sign: fade-in, 8 updates per image, position; scroll arrows: measured conditions,
  positions and order, unaffected by opacities.

Out of scope: viewport tone, colour and flash on windows (not measured).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: window blend mode, version 4.
- `frame-composition`: window blend arithmetic.
- `core-scene`: window drawing rules.

## Impact

- Crates `vkxp-proto`, `vkxp-pixel`, `vkxp-compose-ref`, `vkxp-vk` (compute shader),
  `vkxp-core` (window extraction and state), and tests that pin window pixels.
