# Proposal

## Why

The design record makes `VK_EXT_present_timing` and `VK_KHR_present_wait2` the optional way to
show frames at exactly the game's rate (40 per second by default) on displays whose refresh
rate is no multiple of it, with the timer as the fallback. Only the fallback exists: the
session's timer decides when a frame is submitted, and the swapchain shows it at whatever
refresh comes next, so on a 60 Hz or 144 Hz display the time a frame stays on screen varies
from frame to frame. This is the last open item of milestone M5.

## What Changes

- When the device and surface support `VK_EXT_present_timing`, every present carries a target
  presentation time one frame period after the previous target, in the swapchain's time domain,
  and the presenter reads the reported actual times back.
- When `VK_KHR_present_wait2` is also available, the presenter can wait for a submitted frame
  to reach the display before the next one is submitted, which keeps the queue at one frame.
- Without the extensions nothing changes.
- The extensions are bound by hand, because the Vulkan binding in use predates them.
- A diagnostic output of the achieved frame intervals, on request, for checking the result on
  real hardware.

Out of scope: changing the session's tick timing or the game's logic rate; variable refresh
rate control; frame interpolation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: presentation times.

## Impact

- Crate `vkxp-vk` (device creation, swapchain, presenter) and `vkxp-host` (passes the frame
  period and asks for the statistics); `vkxp` (one option for the diagnostic output).
