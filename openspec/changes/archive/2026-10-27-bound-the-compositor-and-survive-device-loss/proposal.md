# Bound the compositor and survive device loss

## Why

The project review of 2026-10-26 (`target/review-2026-10-26/merged.md`, B5, F4, F5, F8 L10-L12,
L18) found these in `vkrgss-vk` and in how the session treats its errors:

- The only cap on a frame's tile lists is `maxStorageBufferRange / 4` entries (2^32 - 1 on
  desktop drivers). A packet of the largest logical size with about a thousand full-screen ops
  makes the host allocate about 4 GiB and dispatch for minutes; every submission is waited for
  with `u64::MAX`.
- `VkError` cannot say that the device is lost. After `VK_ERROR_DEVICE_LOST` the session
  reports "frame N was rejected" for every frame, for ever, with a black display.
  `release_surface` returns early on a failed `vkDeviceWaitIdle` and leaves the surface marked
  as held although its swapchain is gone.
- `ensure_target` destroys the frame target before its replacement is allocated, so a failed
  allocation after a size change loses the previous frame, against the contract of `render`.
- The staging buffer for bitmap uploads doubles and never shrinks: one upload of 1 GiB pins
  1 GiB of host-visible memory for the rest of the run.
- `present` loses the "unshown frame" flag when both of its attempts meet an out-of-date
  swapchain, so pacing skips that frame.
- The check of update-after-bind descriptors compares the bitmap slots alone with
  `maxPerStageUpdateAfterBindResources`, though the set holds other descriptors too.

## What Changes

- A frame whose binned work exceeds a budget of the backend is refused with a limit error before
  anything is allocated for it; waits for a submission are bounded, and a wait that runs out
  counts as a lost device.
- `VkError` tells a lost device apart; the compositor refuses every later call with that error.
  The session reports a lost device once and ends with a failure, windowed or headless.
- `release_surface` lets the surface go also when idling the device fails.
- The frame target is replaced only once its replacement exists.
- The staging buffer is given back when a run of frames has needed much less of it.
- A present that shows nothing keeps the frame unshown.
- The descriptor limit check counts every descriptor of the set.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: requirements on a work budget, bounded waits and device loss, the
  previous frame on a failed resize, the staging buffer, and descriptor limits.
- `vulkan-presentation`: requirements on an unshown frame and on releasing the surface.
- `host-session`: a requirement on a lost device.

## Impact

`crates/vkrgss-vk` (compositor render, buffers, submit, present, device checks, error, tests),
`crates/vkrgss-host` (`session.rs`: how a compositor error that is a lost device ends the
session; a few lines, plus a session test). `docs/architecture.md` or `docs/decisions.md` where
they describe the waits or the device requirements; `CHANGELOG.md`. No picture changes:
`cargo xtask xarch check` stays identical.
