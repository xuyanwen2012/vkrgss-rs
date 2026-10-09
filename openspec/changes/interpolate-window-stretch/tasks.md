# Tasks

## 1. Stretch (`vkxp-core`)

- [x] 1.1 Implement the bilinear stretch for the background and the cursor centre and commit samples of the corrected measurement. Verify: the scenarios of the two stretch requirements in `specs/core-scene/spec.md`; the earlier window tests still pass.

## 2. Binding (`vkxp-binding`)

- [x] 2.1 Assign `active` and `pause` through the counter-keeping functions. Verify: a Ruby test for the off-and-on scenario and the same for `pause`.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate interpolate-window-stretch --strict` passes.
