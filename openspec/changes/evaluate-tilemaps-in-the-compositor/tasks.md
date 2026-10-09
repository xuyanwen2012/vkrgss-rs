# Tasks

## 1. Protocol (`vkxp-proto`)

- [ ] 1.1 Add table resources and the tilemap layer op and raise the version. Verify: the scenarios of `specs/guest-host-protocol/spec.md`; the reserved-op test now uses the plane op only.

## 2. Compositors (`vkxp-compose-ref`, `vkxp-vk`)

- [ ] 2.1 Evaluate the op in the CPU reference. Verify: the scenarios of the three requirements in `specs/frame-composition/spec.md`.
- [ ] 2.2 Store tables and evaluate the op in the Vulkan compositor. Verify: the CPU-against-GPU tests on llvmpipe over tilemap scenes with every autotile pattern, wrap, priorities and translucent tiles.

## 3. Scene (`vkxp-core`)

- [ ] 3.1 Extract tilemaps as tilemap ops with table uploads on change, keeping the cached composition as the test reference. Verify: the four scenarios of `specs/core-scene/spec.md`.
- [ ] 3.2 Delete the paragraph about the temporary deviation from `docs/decisions.md`. Verify: the Compositor section states the op without it.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate evaluate-tilemaps-in-the-compositor --strict` passes.
