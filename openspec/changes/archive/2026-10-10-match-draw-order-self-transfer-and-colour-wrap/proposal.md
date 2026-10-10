# Proposal

## Why

Four guesses of `vkxp-core`, marked `PLACEHOLDER`, were measured on the original runtime and
three are wrong. Among elements with equal z the original draws strictly in creation order,
and vkxp moves an element to the front when its z changes (42 of 109 observed pairs differ:
any game that takes a sprite to another z and back). A transfer within one bitmap works in
place in the original and smears to the right and downwards, where vkxp copies first (119 of
197 cases differ). A negative red, green or blue of a colour wraps modulo 256 when it is
used, where vkxp takes it as 0. Fractional components and negative zooms are confirmed.

## What Changes

- The order among elements with equal z is the order of creation; a z assignment never
  renews it. This holds for sprites, planes, windows, viewports and tilemaps.
- `blt` and `stretch_blt` from a bitmap onto itself read the live pixels.
- A colour sent to the host wraps negative red, green and blue.
- Confirmed notes are reworded.

Out of scope: `blend_add_translucent` and `blend_sub_translucent` (no such case exists in
the original); planes with a negative zoom (the original hangs; vkxp keeps drawing nothing);
`Viewport#disposed?`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: draw order.
- `core-bitmap`: transfers within one bitmap.
- `core-values`: conversion of colour components.

## Impact

- `crates/vkxp-core` (`scene.rs`, `element.rs`, `extract.rs` only in `sort` and its notes,
  `bitmap.rs`, `color.rs`, notes in `geometry.rs`, their tests and test data), tests of
  `crates/vkxp-binding` and `crates/vkxp-compose-ref` that state the old order.
