# Tasks

## 1. Protocol and reference

- [x] 1.1 The rule set of the tilemap plane op, the protocol version, the one definition of what the numbering of RGSS3 adds; the new field with the value of RGSS2 in every existing literal of the workspace. Verify: the scenarios of `specs/guest-host-protocol/spec.md` except "Documents name the version"; `cargo test -p vkxp-proto --locked` passes and `cargo check --workspace --all-targets --locked` succeeds.
- [x] 1.2 The CPU reference evaluates plane ops of the rule set of RGSS3: tables by flag, the edge under a table, the shadow mask, star tiles, the undefined cases. Verify: the scenarios of `specs/frame-composition/spec.md` except those of "Whole tilemaps of RGSS3 as measured", each as a test that derives its expected pixels from the generated sheets; `git diff --stat` names no file under `crates/vkxp-compose-ref/tests/data/`; `cargo test -p vkxp-compose-ref --locked` passes.

## 2. Topics of the measurement

- [x] 2.1 The topics `tables2`, `shadow3`, `edge1` and `rand4` rebuilt by formula in the fixtures of the reference compositor, and their digests from the model of the measurement in `crates/vkxp-compose-ref/tests/data/tilemap-rgss3-digests.txt`. Verify: the scenarios of "Whole tilemaps of RGSS3 as measured"; the head of the file says which model made the digests; the script that drove the model is under `local/` and not committed; `tests/data/tilemap-rgss2-digests.txt` is unchanged; `cargo test -p vkxp-compose-ref --locked` passes.

## 3. Scene

- [x] 3.1 The tilemap of RGSS3 in the scene: its creation, its rule set in the plane ops, no plane op while it is not visible. Verify: the scenarios of `specs/core-scene/spec.md`; no file of `crates/vkxp-binding` is edited beyond what "Implementation notes" of `design.md` allows; `cargo test -p vkxp-core --locked` and `cargo test -p vkxp-binding --locked` pass.

## 4. Vulkan

- [x] 4.1 The Vulkan compositor evaluates the rule set of RGSS3 and agrees with the reference on the topics, on generated packets and on clipped planes. Verify: the scenarios of `specs/vulkan-compositor/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes, and so does one run of `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk --locked`.

## 5. Documentation

- [x] 5.1 `README.md` and `docs/architecture.md` (the protocol version and what it added), `docs/decisions.md` (section "VX Ace": the undefined cases and the known differences), the documentation of the crates touched. Verify: the scenario "Documents name the version" of `specs/guest-host-protocol/spec.md` and the scenarios of `specs/supported-runtimes/spec.md`; `git diff --stat` against the start of the change names no file under `docs/` but `architecture.md` and `decisions.md`.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-the-tilemap-of-rgss3 --strict` passes.
