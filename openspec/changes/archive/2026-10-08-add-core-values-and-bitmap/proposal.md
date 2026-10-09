# Proposal

## Why

`vkxp-core` is an empty stub, so nothing can hold a picture, draw into it or hand it to a
compositor. The Ruby binding and the scene objects (sprite, window, tilemap) both sit on the
value types and the bitmap, so these come first. They need neither Ruby nor a GPU, which lets
this change run in parallel with the protocol and interpreter changes.

## What Changes

- Value types with RGSS1 semantics in `vkxp-core`: `Color`, `Tone`, `Rect`, `Table` and the
  font attribute value `Font`, including their ranges, equality and the byte layouts of their
  serialized forms as found in the original runtime's data files.
- `Bitmap`: creation, disposal, `clear`, `fill_rect`, `get_pixel`, `set_pixel`, `blt`,
  `stretch_blt`, `hue_change`, `draw_text` and `text_size`, with typed errors for failed
  creation and use after disposal.
- A bitmap arena that gives every bitmap a protocol bitmap id and turns its changes into
  `BitmapDelta`s (create, dirty-rectangle update, destroy) for the bitmaps a frame refers to.
- Text is drawn through a rasteriser interface defined in core. The real font rasteriser
  arrives with `vkxp-text`; tests here use a stand-in.
- New CPU-only arithmetic in `vkxp-pixel` (hue rotation, stretch index mapping), listed with
  its provenance separately from the compositing functions that the shaders mirror.

Out of scope: scene objects, frame extraction, image file decoding, font discovery and glyph
rasterisation, anything Ruby. The existing functions of `vkxp-pixel` and its `provenance()`
list are not changed, so the Vulkan backend is unaffected.

## Capabilities

### New Capabilities

- `core-values`: the value types scripts manipulate (colour, tone, rectangle, table, font
  attributes), their ranges, equality and serialized byte layouts.
- `core-bitmap`: the CPU bitmap, its drawing operations, disposal, text drawing through a
  rasteriser interface, and the bitmap deltas that carry its pixels to the host.

### Modified Capabilities

None.

## Impact

- Crates: `vkxp-core` (new modules), `vkxp-pixel` (one new module; nothing existing changes).
- New development-only dependencies of `vkxp-core`: `vkxp-compose-ref` and `proptest`, to check
  that emitted deltas reproduce the bitmap in a compositor's bitmap store. Development-only
  dependencies do not count for the dependency rules.
- Later changes (scene objects, Ruby binding, text) build on these types.
