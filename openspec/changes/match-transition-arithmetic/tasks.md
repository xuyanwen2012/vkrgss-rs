# Tasks

## 1. Missing mask

- [x] 1.1 Reproduce the missing-mask report end to end, find the cause and fix it. Verify: the mask-file scenario of `specs/host-session/spec.md`.

## 2. Presenter (`vkxp-vk`)

- [x] 2.1 Commit selections of the two sample files as test data and change the mix to the integer formulas with tiling. Verify: tests on llvmpipe for the scenarios of `specs/vulkan-presentation/spec.md`.

## 3. Session (`vkxp-host`)

- [x] 3.1 Compute the step values and pass them on. Verify: the last-step scenario and the existing freeze and transition tests, recomputed where they pinned old values.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-transition-arithmetic --strict` passes.
