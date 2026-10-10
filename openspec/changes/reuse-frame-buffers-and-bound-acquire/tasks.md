# Tasks

## 1. Bounded acquire (`vkxp-vk`)

- [ ] 1.1 Bound the acquire and add the starved state and the test hook. Verify: the scenarios of "Acquiring an image is bounded" in `specs/vulkan-presentation/spec.md`; no `acquire_next_image` call with `u64::MAX` remains.

## 2. Buffer reuse (`vkxp-vk`)

- [ ] 2.1 Replace the per-submission buffers by the five long-lived ones. Verify: the scenarios of `specs/vulkan-compositor/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk` passes.

## 3. Swapchain maintenance (`vkxp-vk`)

- [ ] 3.1 Enable the extension where offered, add present fences, replace the swapchain without idling the device, release images on the failed-draw path, keep the path without the extension. Verify: the scenarios of "Swapchain lifetime".
- [ ] 3.2 Note under "Checked" in `design.md` what llvmpipe offered and the result of the presentation tests on the AMD GPU. Verify: the notes are there.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate reuse-frame-buffers-and-bound-acquire --strict` passes.
