# Proposal

## Why

A survey of twenty-three more games found one that vkxp stops with an error the original does
not raise: its script disposes a sprite and then calls `sprite.bitmap.dispose`. A probe of
every method of disposed sprites, planes, windows, viewports, tilemaps and bitmaps in the
original and in vkxp shows 163 cases of which vkxp differs in four kinds:

- `Sprite#bitmap` and `Tilemap#autotiles` of a disposed element return their object in the
  original; vkxp raises.
- Disposing a viewport disposes the sprites, planes, windows and tilemaps created in it; in
  vkxp they stay alive.
- An element can be created in a disposed viewport in the original and is alive; vkxp raises.

## What Changes

- `Sprite#bitmap` and `Tilemap#autotiles` work on a disposed element.
- Disposing a viewport disposes every element that was created in it.
- Constructors accept a disposed viewport; the new element is alive, refers to that viewport,
  and is not drawn.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-scene-classes`: behaviour around disposed elements and viewports.

## Impact

- Crates `vkxp-binding` (sprite, plane, tilemap and viewport classes; not the window class's
  drawing) and `vkxp-core` (viewport disposal).
