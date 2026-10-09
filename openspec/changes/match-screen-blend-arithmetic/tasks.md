# Tasks

## 1. Arithmetic (`vkxp-pixel`)

- [x] 1.1 Commit the measured samples as test data and implement the formulas with their provenance. Verify: tests for the scenarios of the three requirements in `specs/frame-composition/spec.md`.

## 2. Backends (`vkxp-compose-ref`, `vkxp-vk`)

- [x] 2.1 Mirror the formulas in the GLSL and update dependent tests. Verify: the CPU-against-GPU tests pass on llvmpipe and the workspace tests pass.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-screen-blend-arithmetic --strict` passes.
