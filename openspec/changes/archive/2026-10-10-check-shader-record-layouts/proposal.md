# Proposal

## Why

The Vulkan backend writes each op as 24 words and each tilemap layer as 25 words, and the
compute shader reads them through two GLSL structs. The numeric constants both sides share
are generated from one definition since `drop-plane-op-and-generate-shader-constants`, but
which word holds which field is still written twice: in the Rust encoder and in the struct
declarations. A field moved on one side only produces wrong frames, found by a differential
test that says the frame differs, not where.

## What Changes

- A test checks, field by field, that the shader reads each field of both records from the
  word the Rust encoder writes it to.
- The Rust side names the word index of each field once, and the encoder and the test use
  those names.

Out of scope: generating the GLSL structs; changing either layout; a SPIR-V reflection
library.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-compositor`: the record layouts are checked.

## Impact

- `crates/vkxp-vk` only (`src/records.rs`, `src/shared.rs` if the indices go there, a test
  shader under `shaders/`, a test).
