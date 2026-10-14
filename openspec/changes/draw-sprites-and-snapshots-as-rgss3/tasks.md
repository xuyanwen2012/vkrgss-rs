# Tasks

## 1. Scene of RGSS3

- [x] 1.1 The value of RGSS3 in the rules of a scene, equal to RGSS2 in every place the rules are read (decision 1 of `design.md`); the scenes of RGSS2 that RGSS3 shares, run with it. Verify: the scenarios of "Scene of RGSS3"; `cargo test -p vkxp-core --locked` passes and `cargo check --workspace --all-targets --locked` succeeds.
- [x] 1.2 Turned sprites in viewports by the rule of RGSS3; RGSS1 and RGSS2 keep their shift. Verify: the scenarios of "Turned sprites in viewports of RGSS3" and of "Turned sprites in viewports and with cut source rectangles", each expected number written in its test; `cargo test -p vkxp-core --locked` passes.
- [x] 1.3 The viewport of the screen, the change of the screen's size, moved elements with the provenance in place of "not measured", the sprites at which the original faults. Verify: the scenarios of "Viewport of the screen and a change of its size", "Elements moved between viewports" and "Sprites at which the original of RGSS3 faults"; `cargo test -p vkxp-core --locked` passes; `git diff --stat` against the start of the change names no file of `crates/vkxp-binding` and no existing file under `crates/vkxp-core/tests/data/`.

## 2. Whole scenes of the measurement

- [x] 2.1 The topics `atvp`, `turnedvp`, `aplanevp` and `aheldx` as text scenes with the digests of the model of the measurement, in `sprite-rgss3-turned.txt`, `sprite-rgss3-moved.txt` and `sprite-rgss3-random.txt` under `crates/vkxp-core/tests/data/`; what `test_support::sprite_cases` needs to run them with the rules of RGSS3; compared in the CPU reference. Verify: the scenarios "Turned sprites in viewports", "Moved planes and sprites", "Random scenes held out", "Not the rule of RGSS2" and "Nothing of the original" of "Whole scenes of sprites of RGSS3 as measured"; `../../ref/tools/oracle/measurements/ace-v1-sprite-model.py check <topic> ../../ref/tools/oracle/measurements/ace-v1-sprite-frames/<topic>` prints `wrong 0` in its total for `atvp`, `turnedvp`, `aplanevp` and `aheldx`; the script that drove the model is under `local/` and not committed; `cargo test -p vkxp-core --locked` passes.
- [x] 2.2 The three files composed by both backends. Verify: the scenario "Both backends"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-core -p vkxp-compose-ref -p vkxp-vk --locked` passes; `git diff --stat` against the start of the change names no file under `crates/vkxp-vk/shaders`.

## 3. Input and snapshots

- [x] 3.1 The rules of RGSS3 in the input state: the timing of RGSS2, the call that marks a shown frame, `rgss3-input-samples.txt` from the samples of the measurement. Verify: the scenarios of `specs/core-input/spec.md`; `rgss2-input-samples.txt` is unchanged; the script that converted the samples is under `local/` and not committed; `cargo test -p vkxp-core --locked` passes.
- [x] 3.2 The pixels of a snapshot by the rule of RGSS2 or RGSS3 in the reference compositor; `snap_to_bitmap` of RGSS2 uses it. Verify: the scenarios of `specs/frame-composition/spec.md`; in `crates/vkxp-binding` only the body of `snap_to_bitmap` in `src/graphics.rs` changed and no file under `crates/vkxp-binding/tests/`; `cargo test -p vkxp-compose-ref -p vkxp-core -p vkxp-binding --locked` passes.

## 4. Frozen picture

- [x] 4.1 The rule of the freeze message and the protocol version; the frame rule in every existing literal of the workspace. Verify: the scenarios "Rule travels with the message" and "Version" of `specs/guest-host-protocol/spec.md`; `cargo test -p vkxp-proto --locked` passes and `cargo check --workspace --all-targets --locked` succeeds; in `crates/vkxp-binding/src` only `graphics.rs` changed.
- [x] 4.2 The buffer of RGSS3 in the presenter. Verify: the scenarios of `specs/vulkan-presentation/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes; if `git diff --stat` against the start of the change names a file under `crates/vkxp-vk/shaders`, one run of `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk --locked` passes too.
- [x] 4.3 The session holds by the rule of the message; the headless run across a resize. Verify: the scenarios of `specs/host-session/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host -p vkxp --locked` passes.

## 5. Documentation

- [ ] 5.1 `README.md` and `docs/architecture.md` (the protocol version and what it added), `docs/decisions.md` (section "VX Ace": decisions 1, 5, 8 and 10 with the known differences), the documentation of the modules touched. Verify: the scenario "Documents name the version" of `specs/guest-host-protocol/spec.md` and the scenarios of `specs/supported-runtimes/spec.md`; the text names nothing that the code does not do; `git diff --stat` against the start of the change names no file under `docs/` but `architecture.md` and `decisions.md`.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-sprites-and-snapshots-as-rgss3 --strict` passes.
