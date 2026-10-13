## Why

The scene classes of RGSS2 differ from those of RGSS1 in a handful of names that need no
new pixel rule: an element can be moved to another viewport, a sprite tells its size, a
plane sets its viewport through `viewport(v)`, a fresh window has a contents bitmap, and
`Window#stretch` and `Hangup` do not exist. All of it was measured on the original runtime
of VX (`vx-v1-api-REPORT.md` and `ref/RGSS202E_api.md`, outside the repository). The scripts
of VX use `viewport=` and `Sprite#width` from the first scene on.

## What Changes

- In RGSS2: `Sprite#viewport=`, `Window#viewport=`, `Tilemap#viewport=` and the
  one-argument `Plane#viewport`, which move the element to another viewport or to none.
- In RGSS2: `Sprite#width` and `Sprite#height`; `Plane#visible` reads `true` always; a new
  window has a contents bitmap of 1 by 1; `Window#stretch` and the class `Hangup` are not
  defined.
- A committed list of the methods of the scene classes of RGSS2 with their arities, checked
  by a test, with the methods that later changes add named as pending in that test.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-scene-classes`: the surface of the classes by runtime; moving an element between
  viewports.

## Impact

- `crates/vkxp-binding` (the element classes, the prelude, the method list and its test).
- `crates/vkxp-core` (`scene.rs`, `element.rs`: an element changes its viewport).
- Not touched: `crates/vkxp-binding/src/graphics.rs` and `crates/vkxp-core/src/extract.rs`
  beyond what moving needs: the change `bind-graphics-of-rgss2` runs in parallel there. The
  protocol, the host and the compositors are not touched.
