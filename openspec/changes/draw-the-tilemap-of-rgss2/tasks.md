# Tasks

## 1. Protocol and reference

- [x] 1.1 The op of a tilemap plane of RGSS2 in the protocol, the version, the one definition of numbering and shapes. Verify: the scenarios of `specs/guest-host-protocol/spec.md`; `cargo test -p vkxp-proto --locked` passes.
- [x] 1.2 The CPU reference evaluates the op: tiles, shapes, animation, planes, tables, shadows, flash, blend. Verify: the scenarios of `specs/frame-composition/spec.md`; `cargo test -p vkxp-compose-ref --locked` passes.

## 2. Scene and class

- [x] 2.1 The tilemap of RGSS2 in the scene and its extraction into two plane ops. Verify: the scenarios of `specs/core-scene/spec.md`; `cargo test -p vkxp-core --locked` passes.
- [x] 2.2 The class in RGSS2; the pending lists. Verify: the scenarios of `specs/rgss-scene-classes/spec.md`; `cargo test -p vkxp-binding --locked` passes.

## 3. Vulkan

- [ ] 3.1 The Vulkan compositor evaluates the op and agrees with the reference on the fixtures and on generated packets. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes, and so does `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk --locked`.

## 4. Documentation

- [ ] 4.1 `README.md` (protocol version), `docs/architecture.md` (the op), `docs/compatibility.md` (the tilemap of VX and what vkxp defines where the original is undefined). Verify: the texts name the fields and the version as the code does.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-the-tilemap-of-rgss2 --strict` passes.
