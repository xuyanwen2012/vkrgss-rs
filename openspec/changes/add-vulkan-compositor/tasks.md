# Tasks

## 1. Build setup

- [x] 1.1 Add `ash` and `gpu-allocator` to `[workspace.dependencies]` and to `vkxp-vk`, with `vkxp-proto`, `vkxp-pixel` and `thiserror`; add `vkxp-compose-ref` and `proptest` as dev-dependencies. Verify: `cargo build -p vkxp-vk` succeeds and `cargo xtask deps` still passes.
- [x] 1.2 Add `build.rs` that compiles every shader under `crates/vkxp-vk/shaders/` with `glslc` for Vulkan 1.3 with warnings as errors, runs `spirv-val` on each output, and fails naming the tool when one is missing. Verify: a placeholder compute shader builds; a deliberate syntax error fails the build showing the `glslc` message; `PATH` without `glslc` fails naming `glslc`; then restore.
- [x] 1.3 Document the build and test prerequisites (`glslc`, `spirv-val`, a Vulkan 1.3 device or lavapipe, optional validation layer, `VKXP_TEST_DEVICE`) in `README.md`. Verify: README names each prerequisite and the variable.

## 2. Device

- [x] 2.1 Implement instance and headless device creation with `DeviceChoice` (`Auto`, `Index`, `Name`), the required-feature check and the preference order discrete, integrated, other. Verify: unit tests of the selection function over synthetic device descriptions cover each choice, the preference order, and the error that lists every device with what it lacks.
- [x] 2.2 Add the shared test harness: device from `VKXP_TEST_DEVICE` defaulting to the CPU device, validation layer enabled when installed, a debug-utils messenger recording error-severity messages, and an assertion that the record is empty. Verify: a test creates and drops the device with an empty record; a test that deliberately triggers a validation error is caught by the harness (kept as a test that expects the record to be non-empty).
- [x] 2.3 Set up `gpu-allocator`, the command pool, and teardown order. Verify: creating and dropping the compositor a hundred times in one test reports no validation error and no leaked object.

## 3. Bitmap table

- [x] 3.1 Implement the slot table with `Create`, `Update` and `Destroy`, `R8G8B8A8_UINT` images, staging uploads and the same error kinds as the reference store, plus `Limit` errors for slot capacity and image size. Verify: unit tests mirror the reference store's tests for create, update, destroy, stale generation, out-of-bounds update and length mismatch.
- [x] 3.2 Add a test-only readback of a bitmap's pixels. Verify: a test uploads generated patterns in several partial updates and reads back bytes equal to the reference store's bitmap.

## 4. Shader arithmetic

- [x] 4.1 Write the GLSL include mirroring `vkxp-pixel` function for function, each with a comment naming its Rust counterpart. Verify: `spirv-val` passes in the build and a test asserts every function listed by `vkxp_pixel::provenance()` has a GLSL function of the same name.
- [x] 4.2 Write the GLSL 64-bit helpers and the transform evaluation using 32-bit words only. Verify: a test-only compute shader evaluates a buffer of coefficient and pixel inputs, including the extreme coefficients and randomly generated ones, and every result equals `Transform::source_position`.

## 5. Compositing

- [x] 5.1 Implement packet validation equal to the reference (logical size, deltas in order, op checks, reserved ops) and op-record encoding with clips intersected with the screen. Verify: unit tests for each error kind and index, and for the failed-packet scenarios in `specs/frame-composition/spec.md`.
- [x] 5.2 Implement CPU tile binning as a pure function. Verify: property tests without a device assert that for every pixel the tile's list contains, in op order, exactly the records whose clip contains a pixel of that tile, for sizes that are and are not multiples of 16.
- [x] 5.3 Implement the compute pipeline, descriptor set, per-frame buffers, dispatch and frame readback. Verify: tests for the empty op list, a full-screen clear and one identity quad equal the reference frames.
- [x] 5.4 Implement `handle` so non-frame messages are ignored as in the reference. Verify: a test feeds every `GuestToHost` variant and compares results with the reference.

## 6. Differential tests

- [ ] 6.1 Add the differential helper and run every stream of `vkxp_compose_ref::fixtures::all()` through it. Verify: `cargo test -p vkxp-vk` passes with every fixture byte-identical.
- [ ] 6.2 Add the arithmetic sweeps for each blend mode at several opacities and alphas, colour blend and tone, including a transparent destination. Verify: all sweeps are byte-identical; temporarily changing one constant in the GLSL include makes a sweep fail, then restore.
- [ ] 6.3 Add `proptest` strategies for valid packet sequences and for sequences with injected invalid deltas and ops. Verify: both properties pass with a fixed case count; temporarily reversing the tile list order makes the valid-sequence property fail, then restore.
- [ ] 6.4 Add the targeted cases: several thousand ops on one pixel, an op entirely off screen, logical size 641x479, both limit errors and recovery after them. Verify: each is a named test that passes.

## 7. Integration check

- [ ] 7.1 Run the differential tests on a hardware device. Verify: `VKXP_TEST_DEVICE=<name> cargo test -p vkxp-vk` passes on at least one hardware GPU, and the device name and driver version are recorded in the commit message.
- [ ] 7.2 Run the full gate. Verify: `cargo xtask gate` exits 0 and `openspec validate add-vulkan-compositor --strict` passes.
