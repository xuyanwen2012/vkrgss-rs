# Tasks

## 1. Workspace skeleton

- [x] 1.1 Create the root `Cargo.toml` workspace (edition 2024, shared package metadata, lints, dependency table), `rust-toolchain.toml` pinned to the installed stable, and `LICENSE` (MIT). Verify: `cargo metadata --no-deps` succeeds.
- [x] 1.2 Create the thirteen crates under `crates/` as stubs with crate-level docs stating side and responsibility from `docs/decisions.md`. Verify: `cargo build --workspace` has no warnings.
- [x] 1.3 Add `#![forbid(unsafe_code)]` to `vkxp-proto`, `vkxp-pixel`, `vkxp-core`, `vkxp-compose-ref`. Verify: a temporary `unsafe {}` in `vkxp-core` fails to compile, then remove it.

## 2. Gate and dependency rules

- [x] 2.1 Add an `xtask` crate with a `gate` command running fmt check, clippy `-D warnings`, workspace tests and `openspec validate --all`, reporting which step failed. Verify: `cargo xtask gate` exits 0; breaking formatting makes it exit non-zero naming the fmt step.
- [x] 2.2 Add a dependency-direction test over `cargo metadata`: guest crates must not reach host crates; `vkxp-core` only reaches its allow-list. Verify: unit tests feed it a synthetic graph with a forbidden edge and assert the reported path; the real workspace passes.
- [x] 2.3 Document the gate and the rules in `README.md`. Verify: README names the command and both rules.

## 3. Protocol types

- [x] 3.1 Implement fixed-point, rectangle, colour, tone and bitmap-id (index plus generation) types in `vkxp-proto`. Verify: unit tests for fixed-point conversion and id equality.
- [x] 3.2 Implement `BitmapDelta`, `Op` (`Clear`, `Quad`, `RectEffect`, reserved `Plane`, `TilemapLayer`), `FramePacket`, `AudioCommand`, `InputSnapshot`, `GuestToHost`, `HostToGuest`. Verify: a compile-time or test check that no message field is `f32`/`f64`.
- [x] 3.3 Implement versioned encode/decode with magic and protocol version. Verify: property tests for round trip and deterministic bytes; a test that an unsupported version is rejected with both versions in the error.
- [x] 3.4 Implement input-recording read/write on top of 3.3. Verify: a recorded snapshot sequence round-trips through a temp file.

## 4. Pixel arithmetic

- [x] 4.1 Implement normal, additive and subtractive blend with opacity, colour blend and tone in `vkxp-pixel`, each with a documented formula and a provenance marker. Verify: unit tests for zero opacity, zero alpha, saturation and neutral tone.
- [x] 4.2 Expose a provenance listing. Verify: a test asserts every public arithmetic function is listed and currently marked as placeholder.

## 5. CPU reference compositor

- [x] 5.1 Implement the bitmap store driven by `BitmapDelta`, including stale-id detection. Verify: tests for create/update/destroy and for a reused slot with an old generation.
- [x] 5.2 Implement per-pixel ordered evaluation of `Clear`, `Quad` and `RectEffect` with clip rectangles and nearest sampling. Verify: one test per scenario in `specs/frame-composition/spec.md`.
- [x] 5.3 Return a typed unsupported-op error for reserved ops. Verify: a test with a `Plane` op gets the error and no frame.
- [x] 5.4 Add the headless example: encoded messages in, PNG out. Verify: running it on a generated fixture writes a PNG whose bytes match the committed expected image.
- [x] 5.5 Add generated fixtures (checkerboard, gradient, alpha shape; overlap, clip, each blend mode, rectangle effect) with committed expected PNGs. Verify: `cargo test -p vkxp-compose-ref` compares all of them byte for byte and passes twice in a row.

## 6. Integration check

- [ ] 6.1 Run the full gate on the finished tree. Verify: `cargo xtask gate` exits 0 and `openspec validate define-architecture` passes.
