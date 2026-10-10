# Design

## Context

- `crates/vkxp-proto/src/frame.rs`: `Op::Plane { clip }`, `OpKind::Plane`,
  `OpKind::is_reserved`. `codec.rs`: `PROTOCOL_VERSION = 6`, one tag per op kind.
- Both backends return an unsupported-op error for a reserved kind; tests in
  `vkxp-compose-ref` and `vkxp-vk` send a `Plane` op to check it.
- `crates/vkxp-vk/src/records.rs`: `op_kind`, `sampling_kind`, `blend_mode`, `RECORD_WORDS`,
  `TILEMAP_WORDS`, `NO_BITMAP`, `WHOLE_TILES`, each with a comment naming its twin in
  `shaders/compose.comp`. `present.rs` and `present.frag` share flag bits the same way.
- `crates/vkxp-vk/build.rs` compiles every shader with `glslc`; `*.glsl` files are includes.

## Decisions

### The op

- Remove `Op::Plane`, `OpKind::Plane` and `OpKind::is_reserved`. The codec tags of the other
  kinds keep their values; the tag of `Plane` is not reused. `PROTOCOL_VERSION` becomes 7.
- The unsupported-op error of a backend goes if nothing else can produce it; if a variant is
  part of a public error type that other code matches on, removing it is part of this change.
- Tests that existed to show the rejection are removed with it; tests that used a `Plane` op
  as "some op" use another.
- `docs/decisions.md` lists planes among what the guest emits; it stays true (they are quads).
  The README's protocol description is brought in line.

### The constants

- A file of plain `pub const` items, `crates/vkxp-vk/src/shared.rs`, with no imports, holds
  every number both sides use. The crate uses it as a module; `build.rs` includes the same
  file with `include!` and writes `shared.glsl` into `OUT_DIR`, one `const uint NAME = ..u;`
  per item, from a list in `build.rs` that names each constant once.
- `glslc` gets `OUT_DIR` as an include directory; `compose.comp` and `present.frag` include
  `shared.glsl` and lose their own definitions of those constants.
- `blend_mode` maps the protocol's enum to the shared numbers, as today.
- A unit test fails when the list in `build.rs` and the file drift apart (a constant in the
  file that the list does not name), by reading the generated text.

## Risks / Trade-offs

- [A generated include is harder to read in an editor] → The file is short and its source is
  one Rust file named in a comment at the include.

## Follow-ups

- Generating the record layouts too.
- The codec has no explicit tag per op kind: postcard encodes an enum variant by its position,
  so removing `Plane` moves `TilemapLayer` from position 4 to 3. Keeping the old positions
  needs a hand-written wire form of `Op`, which this design does not name; it was not built.
  Nothing can observe the difference, because streams of version 6 are rejected by the
  version check before any op is decoded.
- The fake display of `vkxp-host` (`src/testing.rs`) and the host tests used the reserved op
  as "a packet every compositor rejects". They now use a quad of a bitmap that does not
  exist, which both compositors and the fake display reject.
