# Design

## Context

- `docs/decisions.md`, Compositor: "A tilemap layer is an op type evaluated in the shader, not
  expanded into per-tile quads", followed by a paragraph that records the present CPU cache as
  a temporary deviation.
- `vkxp_proto`: `Op::TilemapLayer { clip }` exists as a reserved kind that backends reject;
  `FramePacket` has `bitmaps: Vec<BitmapDelta>` and `ops`; `PROTOCOL_VERSION` is 5.
- `vkxp-core`'s `tilemap.rs` keeps, per tilemap, cached bitmaps per priority and map layer,
  composed with `blt` from the tileset and autotiles for the visible cells, re-made when the
  scroll crosses a tile boundary, a table changes or an autotile animates, and drawn as
  quads: priority 0 at z 0, priorities 1 to 5 per map row at the row's z. The flash overlay is
  one more cached bitmap. Tables are `SharedTable`s with a revision counter. The autotile
  pattern table (48 patterns of four quarters) is in that file and in the spec.
- Measured on the original: every tile is blended on the screen by itself with the sprite
  formula, in ascending priority and, within one priority, in layer order; a tile's
  translucent texels therefore meet whatever was drawn before them. The present cache layout
  reproduces this and is pixel-identical to the original on the regression games.
- The Vulkan compositor bins ops into 16 by 16 tiles on the CPU and evaluates them in one
  compute dispatch with `texelFetch` from a bindless array of bitmaps, integer arithmetic only;
  the CPU reference evaluates the same list and both must agree bit for bit.

## Decisions

- Protocol: `FramePacket` gains `tables: Vec<TableDelta>` (create or replace with dimensions
  and data, destroy), applied before the ops; `TableId` is a number space of its own. The op
  is `TilemapLayer { clip, tileset, autotiles: [Option<BitmapId>; 7], map, priorities, origin,
  layer, priority, animation_step }`. Version 6.
- Evaluation is exactly the three requirements of `specs/frame-composition`. The pattern table
  is a constant shared by the CPU reference and, as an array, by the shader; a test compares
  the two copies with the one in `vkxp-core`.
- Vulkan side: tables are stored in one storage buffer with an offset and dimensions per
  table (16-bit values widened to 32-bit integers), which needs no new device feature; ops of
  this kind are binned by their clip like the others.
- Core: per tilemap and frame, the visible cells are scanned once to find which priorities
  occur in which rows and layers, and only those ops are emitted, so an ordinary map costs a
  handful of ops. The op for a row is clipped to that row's screen band intersected with the
  viewport. Draw order and z values are those of the quads they replace.
- The cached-bitmap composition is removed from the drawing path but its code is kept in the
  tests of `vkxp-core` as the reference the new extraction is compared with, frame for frame,
  on the existing tilemap scenes.
- The flash overlay is unchanged.
- `docs/decisions.md`: the paragraph about the temporary deviation is deleted.

## Risks / Trade-offs

- [A per-pixel table lookup for every map pixel and layer] → Three lookups and a few integer
  operations per layer; the binning keeps ops off tiles they do not touch.
- [Pixel regressions] → The comparison with the cached implementation in the tests, and the
  coordinator's regression run against the original on seven games after the merge.

## Follow-ups

- The autotile requirement names only sheets of quarters. The cached composition also drew a
  sheet at most 32 pixels high as whole tiles, one per frame of 32 pixels, and games use such
  sheets, so the op does the same in both compositors to keep the pictures. Should the
  requirement say so?
- A priority outside 0 to 5 was brought into that range by the cached composition (a
  placeholder in `vkxp-core`). The op compares entry and priority as the requirement says, and
  `vkxp-core` sends the priorities already brought into range, so nothing changes on screen.
  What the original runtime does with such a priority is still unmeasured.
- `tileset` in the op is optional, like the autotiles: a tilemap without a tileset can still
  show autotiles.
- The origin of the op is a pair of `i32`. An origin that the viewport's offset pushes beyond
  that range is sent as the equivalent origin inside the map.
- A written table is sent whole, as the design says. A script that writes to a large map every
  frame pays for the whole map every frame; a partial update would be a protocol addition.
- All tables share one storage buffer, so their total size is bounded by the device's storage
  buffer range and reported as a limit beyond it.
