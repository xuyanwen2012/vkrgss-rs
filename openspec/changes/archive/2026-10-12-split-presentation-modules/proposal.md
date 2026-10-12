# Split the presentation modules of the Vulkan backend

## Why

Three files of `vkxp-vk` have grown past what can be read in one sitting:
`compositor/present.rs` (2543 lines), `device.rs` (1744) and `compositor.rs` (1480).
`present.rs` holds the scaling arithmetic, the choice of the surface format, the swapchain,
the presenter, a block of about 1270 lines of `impl VkCompositor`, and 420 lines of tests.
`device.rs` is tests from line 1256 on. In `present.rs` and `compositor.rs` about fifty
`#[cfg(test)]` attributes sit on single fields and statements of the production code, so the
code that runs in a game and the bookkeeping of the tests are read together.

## What Changes

- The unit tests of the three files move into files of their own.
- `compositor/present.rs` and `compositor.rs` are split into modules by concern.
- What the production code records only for tests is gathered in one probe type per module.
- Nothing a caller of `vkxp-vk` can see changes, and no frame changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: the layout of the backend's source is stated.

## Impact

`crates/vkxp-vk/src` only. No shader, no other crate, no dependency, no picture.
