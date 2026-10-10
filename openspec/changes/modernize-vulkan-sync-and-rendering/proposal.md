# Proposal

## Why

The Vulkan backend requires Vulkan 1.3 but is written against the 1.0 interfaces: image
layout changes go through `vkCmdPipelineBarrier` with stage masks guessed from the layout, and
the presentation pass uses render pass and framebuffer objects, one framebuffer per swapchain
image, with a cache of render passes per surface format. The guessed masks are wrong for one
case that exists today: an image in the shader-read layout is taken to be read by the compute
stage only, while the presentation pass reads the held frame and the transition mask from the
fragment stage, and the composed frame goes from the compute dispatch to the fragment pass
without a barrier. The output is right only because every submission is waited for before the
next begins.

## What Changes

- Barriers use the synchronization2 interface, with the stages and accesses stated where the
  barrier is recorded instead of derived from the layout.
- The hand-over of the composed frame from the compute dispatch to the presentation pass, and
  the fragment stage's reads of the held frame and the mask, are covered by barriers that
  name those stages.
- The presentation pass uses dynamic rendering; the render pass objects, their cache and the
  framebuffers go.
- An image creation flag nothing uses (`MUTABLE_FORMAT` on the frame target) is dropped.

Out of scope: how many submissions a frame takes and what is waited for, buffer reuse,
swapchain maintenance, the acquire timeout, surface formats (later changes); any change to
pixels.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: the device features the backend enables; clean synchronisation.
- `vulkan-presentation`: no render pass or framebuffer objects.

## Impact

- `crates/vkxp-vk` only. Another change edits one constant in `src/records.rs` and the test
  file `src/tests/tilemap.rs` at the same time; leave those two places alone.
