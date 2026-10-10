# Proposal

## Why

The frame is composed in non-linear 8-bit values and must reach the screen unchanged. On a
surface that offers a `UNORM` format that is what happens. On a surface that offers only sRGB
formats the presentation shader applies the inverse of the sRGB encoding in floating point so
that the format's encoding gives the value back; that round trip is not guaranteed to return
the same 8-bit value for every input. A surface that offers neither kind is taken as it comes
without a look at its colour space.

## What Changes

- On an sRGB-only surface the swapchain is created so that its images can be viewed as the
  matching `UNORM` format, and the presentation pass renders through that view. The values
  written are the values stored; the shader's inverse transfer is no longer used there.
- Where the device cannot do that, the present path of today stays as the fallback.
- The last-resort choice of a surface format prefers one in the sRGB non-linear colour space.

Out of scope: wide-gamut or HDR output; the off-screen scaling path (it already targets a
`UNORM` image); pacing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: exact values on sRGB surfaces.

## Impact

- `crates/vkxp-vk` only.
