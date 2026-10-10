# Design

## Context

- `crates/vkxp-vk/src/records.rs` builds `[u32; RECORD_WORDS]` and `[u32; TILEMAP_WORDS]`
  values with positions written as literals or by order.
- `shaders/compose.comp` declares `OpRecord` (with a nested `Transform`) and `TilemapRecord`.
- The crate already has a test-only compute shader and runner (`shaders/transform_test.comp`,
  `src/compositor/test_compute.rs`) that feeds a buffer to a shader and reads a buffer back.
- `shared.rs` holds constants that `build.rs` turns into `shared.glsl`.

## Decisions

- Name the word index of every field of both records as constants in Rust
  (`OP_KIND = 0`, ...), in the module that holds the encoder; the encoder writes through
  them. They are Rust-only: the shader keeps reading through its structs.
- The struct declarations move from `compose.comp` into an include file
  (`shaders/records.glsl`) so that a test shader reads the very same declarations.
- A test shader binds one op record and one tilemap record whose word `i` holds a value
  derived from `i` (distinct for every word), and writes every field of both structs to an
  output buffer in a fixed order. The test compares each output with the value of the word
  that the Rust constant of that field names. Array fields (the autotile slots and frames)
  are checked element by element.
- The test's list of fields is written once, as pairs of the Rust constant and the output
  position; a field added to a struct and not to the list is caught by a count: the outputs
  must cover every word of both records exactly once.
- Frames do not change: the differential tests stay as they are.

## Risks / Trade-offs

- [The test shader's field list is a third copy] → It is, but a wrong entry fails the test
  instead of drawing a wrong frame, and the coverage count catches a missing one.

## Follow-ups

- Generating the structs from the Rust definition.
