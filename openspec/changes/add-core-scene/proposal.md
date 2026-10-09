# Proposal

## Why

`vkxp-core` can hold and draw bitmaps but has nothing that puts them on the screen. Scripts
describe a frame through sprites, viewports and windows with positions, z order, opacity,
tone and so on, and ask about buttons through `Input`. That scene model, and its translation
into the protocol's frame packet, is the last piece of core the Ruby binding needs to show a
title screen.

## What Changes

- A scene arena in `vkxp-core` with sprites, viewports and windows, their RGSS1 properties,
  disposal, and the draw order (z, then recency).
- Frame extraction: the scene plus the bitmap arena become a protocol `FramePacket` (bitmap
  deltas for the bitmaps used, then quads and rectangle effects in draw order), carrying the
  frame rate.
- Sprite geometry (origin, zoom, rotation, mirror, source rectangle) converted once to the
  protocol's fixed-point transform; sprite opacity, blend type, colour, tone and flash.
- Viewport clipping, scrolling and its tone, colour and flash effect.
- Window drawing from a window skin: frame, background, contents, cursor with its blink, pause
  sign and scroll arrows.
- Graphics bookkeeping (frame count, frame rate) and the `Input` state logic (press, trigger,
  repeat, four- and eight-way direction) fed by protocol input snapshots.
- Planes and tilemaps exist as property holders that draw nothing yet, so scripts that create
  them do not fail.

Out of scope: plane and tilemap drawing (they need new protocol ops), bush depth, sending
packets or waiting for ticks (the binding does that), anything Ruby.

## Capabilities

### New Capabilities

- `core-scene`: sprites, viewports and windows, their draw order, and the extraction of a frame
  packet from them.
- `core-input`: button state derived from input snapshots: pressed, triggered, repeated, and
  direction queries.

### Modified Capabilities

None.

## Impact

- Crate `vkxp-core` (new modules). The allow-list of `vkxp-core` in `xtask/src/deps.rs` gains
  `libm`, a pure-Rust mathematics crate, so that rotation gives the same result on every
  platform; `Cargo.toml` and `Cargo.lock` change accordingly.
- Tests use `vkxp-compose-ref`, already a development-only dependency of `vkxp-core`, to assert
  on rendered pixels.
- The Ruby binding builds directly on this change.
