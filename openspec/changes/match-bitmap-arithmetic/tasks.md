# Tasks

## 1. Arithmetic (`vkxp-pixel`)

- [x] 1.1 Commit selections of the three sample files as test data and implement the `blt` blend with its provenance. Verify: the two scenarios of the blend requirement in `specs/core-bitmap/spec.md`.
- [x] 1.2 Implement the stretch step and the grey tone. Verify: the sixteen-to-twenty-four scenario and the scenario of `specs/frame-composition/spec.md`.

## 2. Bitmap operations and values (`vkxp-core`)

- [x] 2.1 Use the new blend in `blt` and `stretch_blt`, with mirroring and the clipping rules. Verify: the measured-samples scenario of the stretch requirement, including the clipped and mirrored cases.
- [x] 2.2 Change colour storage and the bounds of `Color` and `Tone`. Verify: the two scenarios of the colour requirement, and the Ruby-level tests of `vkxp-binding` for those classes.

## 3. Backends (`vkxp-compose-ref`, `vkxp-vk`)

- [ ] 3.1 Mirror the grey tone in the GLSL and update dependent tests. Verify: the CPU-against-GPU tests pass on llvmpipe.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-bitmap-arithmetic --strict` passes.
