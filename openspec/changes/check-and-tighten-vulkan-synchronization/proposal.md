# Check and tighten the synchronization of the Vulkan backend

## Why

"Order of work" in `docs/decisions.md` places the Vulkan items of the modernization audit
of 2026-10-14 between the platform steps. Three of them belong together:

- The tests run with the validation layer, but its synchronization checks are off by
  default, so a missing or wrong barrier is not reported (audit item 4).
- Submissions use `vkQueueSubmit` and a fence that is waited for and reset each time, while
  the device already enables synchronization2; `vkQueueSubmit2` with a timeline semaphore
  states the stages of each wait and removes the reset (item 6).
- Bitmap uploads record one barrier per image and direction; one `DependencyInfo` for all
  images before the copies and one after does the same with fewer commands, and an image
  replaced whole can start from `UNDEFINED` (item 7).

None changes a composed pixel; the differential tests against the CPU reference check that.

## What Changes

- The tests of `vkrgss-vk` enable the layer's synchronization validation through
  `VK_EXT_layer_settings` when the layer offers it; a hazard it reports fails the test as
  any validation error does. Hazards it finds are fixed.
- Submissions and the wait for them use `vkQueueSubmit2` and a timeline semaphore instead of
  `vkQueueSubmit` and a fence that is reset. The fence of a present (swapchain
  maintenance) stays, as the specification wants it.
- Uploads of bitmaps batch their barriers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: validation includes synchronization; submissions and uploads.

## Impact

`crates/vkrgss-vk` only (`validation.rs`, `device.rs`, `compositor/submit.rs`,
`compositor/setup.rs`, `compositor/present/show.rs`, `resources.rs`, `bitmaps.rs`, tests),
`docs/development.md` if it describes validation, `CHANGELOG.md`.
