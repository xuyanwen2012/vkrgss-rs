# Tasks

## 1. Selection (`vkxp-vk`)

- [x] 1.1 Enable the extension where offered and extend the format choice. Verify: the scenarios "sRGB only, views supported", "sRGB only, views not supported" and "Unknown formats" of `specs/vulkan-presentation/spec.md` as unit tests.

## 2. Swapchain and pass (`vkxp-vk`)

- [x] 2.1 Create the swapchain with the format list and render through the `UNORM` view. Verify: the scenario "Forced path under validation"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk` passes.
- [x] 2.2 Write the notes under "Checked" in `design.md`. Verify: the notes are there.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate present-exactly-on-srgb-surfaces --strict` passes.
