# Tasks

## 1. Blend mode (`vkxp-proto`, `vkxp-pixel`, `vkxp-compose-ref`, `vkxp-vk`)

- [x] 1.1 Add the window blend mode with its arithmetic and the odd-pixel rule to the protocol, the CPU reference and the shader. Verify: the scenarios of `specs/guest-host-protocol/spec.md` and `specs/frame-composition/spec.md`, and the CPU-against-GPU tests on llvmpipe.

## 2. Windows (`vkxp-core`)

- [ ] 2.1 Commit a selection of the samples as test data and implement background and frame. Verify: the scenarios of the background and frame requirements in `specs/core-scene/spec.md`.
- [ ] 2.2 Implement contents, arrows and cursor. Verify: the scenarios of those two requirements.
- [ ] 2.3 Implement the pause sign. Verify: the fade-in scenario.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-window-rendering --strict` passes.
