# Tasks

## 1. Protocol (`vkxp-proto`)

- [x] 1.1 Add the sampling variants and raise the version. Verify: the two scenarios of `specs/guest-host-protocol/spec.md`.

## 2. Compositors (`vkxp-compose-ref`, `vkxp-vk`)

- [x] 2.1 Evaluate stretch quads in the CPU reference and in the compute shader. Verify: the scenarios of `specs/frame-composition/spec.md` and the CPU-against-GPU tests on llvmpipe, including destination rectangles far outside the screen and widths above 4096.

## 3. Scene (`vkxp-core`)

- [ ] 3.1 Extract unrotated sprites as stretch quads and rotated ones with the integer-offset rule. Verify: the scenarios of the two sprite requirements in `specs/core-scene/spec.md`.
- [ ] 3.2 Extract planes as tiled stretch quads. Verify: the plane scenario.
- [ ] 3.3 Apply the bush and flash rules. Verify: the two scenarios of that requirement.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-sprite-and-plane-sampling --strict` passes.
