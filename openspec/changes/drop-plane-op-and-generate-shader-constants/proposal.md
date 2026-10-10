# Proposal

## Why

- The protocol has an op kind, `Plane`, that was reserved at the start and never used: planes
  are sent as stretched quads. Both backends carry code and tests whose only job is to reject
  it, and a reader of the protocol has to learn that it does nothing.
- The numbers that the Vulkan backend and its compute shader must agree on (op kinds, blend
  modes, sampling kinds, record sizes, the two tilemap markers) are written twice, in
  `src/records.rs` and in `shaders/compose.comp`, and kept equal by comments.

## What Changes

- **BREAKING** (protocol): the op kind `Plane` and the notion of a reserved op kind are
  removed. The protocol version becomes 7; streams of version 6 are rejected like any other
  unsupported version.
- The constants shared by the backend and its shaders are defined once in Rust; the build
  writes a GLSL file from them that the shaders include.

Out of scope: the layout of the records themselves (still mirrored by hand, and covered by
the differential tests); the tile numbering table of the shader; a shader language other
than GLSL.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: no reserved op kinds.
- `vulkan-compositor`: shared constants come from one definition.

## Impact

- `crates/vkxp-proto` (the op, its kind, the codec, the version), `crates/vkxp-compose-ref`
  and `crates/vkxp-vk` (the rejection paths and their tests; the build script and shaders),
  `crates/vkxp-core` and `crates/vkxp-host` only where a test or a match names the kind.
- Recordings made with protocol version 6 no longer load; none are committed except as test
  data that the tests write themselves.
