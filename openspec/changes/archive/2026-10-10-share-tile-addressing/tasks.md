# Tasks

## 1. Pin the pictures (`vkxp-core`)

- [x] 1.1 Commit an expectation for each tilemap test scene the cached comparison covers, asserted by the test. Verify: `cargo test -p vkxp-core` passes with the comparison still in place; changing one tile of a scene makes its test fail.

## 2. One definition (`vkxp-proto`, `vkxp-core`, `vkxp-compose-ref`, `vkxp-pixel`)

- [x] 2.1 Add the tile numbering to `vkxp-proto` with unit tests and use it in both crates. Verify: the scenarios of `specs/frame-composition/spec.md`; `cargo test -p vkxp-proto -p vkxp-core -p vkxp-compose-ref`.
- [x] 2.2 Define `MAX_LOGICAL_DIMENSION` once and settle the two `stretch_index` functions as `design.md` says. Verify: `cargo xtask deps` passes; one definition of the constant in the workspace.
- [x] 2.3 Point the shader table test of `vkxp-vk` at the shared table. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk` passes.

## 3. Remove the cached implementation (`vkxp-core`)

- [x] 3.1 Remove `tilemap/cached.rs`, the `#[cfg(test)]` branches in production functions, the test-only field and the hook, and what `tiles.rs` no longer needs. Verify: the "Same pictures as before" scenario of `specs/core-scene/spec.md`; no `#[cfg(test)]` remains inside a non-test function of `tilemap.rs`.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate share-tile-addressing --strict` passes.
