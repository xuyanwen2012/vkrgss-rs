# Tasks

## 1. Presenter (`vkxp-vk`)

- [x] 1.1 Add the mask input, progress and vagueness to the present pass and its off-screen variant. Verify: tests on llvmpipe for the three scenarios of `specs/vulkan-presentation/spec.md`, and the existing mix tests still pass.

## 2. Session (`vkxp-host`)

- [x] 2.1 Carry mask and vagueness through the transition state and remove the "not implemented" report. Verify: tests for the masked-transition and unknown-mask scenarios of `specs/host-session/spec.md`; the other scenarios still pass.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-transition-pictures --strict` passes.
