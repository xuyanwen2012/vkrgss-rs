# Proposal

## Why

Three things in the Vulkan backend cost more than they need to or can stop the game:

- Acquiring a swapchain image waits without a bound. On a compositor that stops handing out
  images while a window is hidden or the session is locked, the game stops with it: logic,
  music fades and timers freeze until the window is shown again.
- A frame creates and destroys up to five buffers (bitmap staging, ops, tile heads, tile
  indices, tilemap records) and rewrites their descriptors, although one submission is in
  flight at a time and the sizes change little from frame to frame.
- Replacing the swapchain idles the whole device, because nothing else tells when the
  presentation engine has let go of the old swapchain's semaphores.
  `VK_KHR/EXT_swapchain_maintenance1` is listed as optional in `docs/decisions.md` and is not
  used.

## What Changes

- The acquire has a bound. When no image arrives in time the frame is rendered and not
  shown, the call reports "not presented" without an error, and the game goes on.
- The per-frame buffers are kept and reused; one is replaced by a larger one when a frame
  needs more.
- Where the device has swapchain maintenance, presents carry a fence, the swapchain is
  replaced without idling the device, and images acquired for a draw that failed are
  released. Without the extension the backend behaves as it does today.

Out of scope: the number of submissions per frame, fences per submission, surface formats,
pacing; `vkxp-host` (it already ignores whether a present showed an image); any change to
pixels.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: bounded acquire; swapchain replacement with and without swapchain
  maintenance.
- `vulkan-compositor`: per-frame buffers are reused.

## Impact

- `crates/vkxp-vk` only.
- `README.md` where it lists what the backend uses of the device, if it does.
