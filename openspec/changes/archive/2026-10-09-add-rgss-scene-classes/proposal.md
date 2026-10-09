# Proposal

## Why

Scripts put things on the screen through `Sprite`, `Viewport`, `Window`, `Plane` and `Tilemap`
objects. `vkxp-core` implements what those do, and the binding can wrap Rust state in Ruby
objects, but the five classes themselves do not exist yet, so no script can show anything.

## What Changes

- The Ruby classes `Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap` and the autotile list
  `TilemapAutotiles`, with every method of the original runtime at the original arity, on top
  of core's scene.
- Properties that are objects in the original (`color`, `tone`, `src_rect`, `rect`,
  `cursor_rect`) return the element's own object, so changing it changes the element; assigning
  copies the values.
- `bitmap`, `windowskin`, `contents`, `tileset`, the autotiles and the tilemap's tables keep the
  Ruby objects they were given, alive and identical on reading back.
- Disposal with the original's error messages, refusal of `clone` and `dup`, and freeing of
  unreachable elements by the garbage collector.
- A test helper that renders the current scene with the reference compositor and lets Ruby
  tests ask for pixels.

Out of scope: `Graphics`, `Input`, `Audio` and running a game (a parallel change), bush depth
and tilemap flash data beyond storing the values.

## Capabilities

### New Capabilities

- `rgss-scene-classes`: the Ruby classes `Sprite`, `Viewport`, `Window`, `Plane` and `Tilemap`.

### Modified Capabilities

None.

## Impact

- Crate `vkxp-binding`: new modules (`sprite`, `viewport`, `window`, `plane`, `tilemap`) and
  tests; the wrapper for `Color`, `Tone` and `Rect` gains owners; `Table` objects become
  shareable with tilemaps.
- After this change the arity test covers the whole method list of the original runtime.
