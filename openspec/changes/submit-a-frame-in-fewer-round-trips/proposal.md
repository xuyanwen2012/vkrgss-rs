# Submit a frame in fewer round trips

## Why

The Vulkan backend is synchronous by design: every method submits its work and blocks until
the device has finished it (`compositor.rs:103-104`, "no Vulkan object is ever in use by the
device when a method returns"). A shown frame therefore makes the host wait more than once:

- `render` records the uploads and the compose dispatch and waits in `submit_and_wait`
  (`compositor/submit.rs:154`);
- `present` records the presentation pass and waits again in `submit_with`
  (`compositor/present/show.rs:283-298`, through `submit.rs:220-260`);
- a freeze (`present/held.rs`) clears the held image and copies the frame into it in two
  submissions, each waited for (`held.rs:378`, `:431`).

The window session calls `render` and then `present` for every frame
(`vkrgss-host/src/windowed.rs:385-387`), so the processor idles once while the device
composes and once while it draws, although nothing on the host needs the composed frame
before the presentation pass is submitted. Both reviews of 2026-10-26 named the extra host
round trips as a cost of the synchronous design.

## What Changes

- `render` submits the uploads and the compose dispatch without waiting for them. The next
  method that needs that work finished, or the resources it uses, waits for it first.
- `present` submits its pass after the compose submission on the same queue and waits once,
  for both.
- A freeze records its clear and copy into one submission and waits once.
- The contract of `VkCompositor` is restated: at most the submission of the last `render` is
  in flight when a method returns, and every method that records, reads back or destroys
  waits for it first; dropping the compositor waits for it.
- Frames, errors and the held-frame contract stay as they are; frame times before and after
  are recorded.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: how many times the host waits for the device per shown frame, and the
  restated contract.

## Impact

`crates/vkrgss-vk/src` only (`compositor.rs`, `compositor/submit.rs`, `compositor/render.rs`,
`compositor/buffers.rs`, `compositor/present/{show,held,readback,overlay,lifecycle}.rs`,
`compositor/measure.rs`, tests). It runs after `bound-the-compositor-and-survive-device-loss`,
which also edits `vkrgss-vk`, and assumes its bounded wait and device-loss error. No other
crate, no shader, no protocol, no picture changes.
