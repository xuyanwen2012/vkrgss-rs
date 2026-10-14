# Bind the Tilemap of RGSS3

## Why

The tilemap of RGSS3 is drawn in `vkxp-core` since `draw-the-tilemap-of-rgss3`: four
layers with the shadows in the fourth, tables by the flags of the tileset, star tiles of
the lower layers, a tilemap that `visible = false` hides. No script reaches it. A game of
RGSS3 still gets the class of RGSS2: it has `passages` where the standard scripts of VX Ace
call `flags=`, so a map scene ends with `NoMethodError`, and what it would draw follows the
rules of VX. Milestone A2, walking on a map, needs the class.

## What Changes

- In a game of RGSS3 `Tilemap.new` makes a tilemap of the scene through
  `Scene::create_tilemap_rgss3`, so its planes are drawn by the rule set of RGSS3.
- The class has the methods measured on the original and listed in `RGSS301_api.md`:
  `flags` and `flags=` in the place of `passages` and `passages=`, with the same rules of
  reference and of `nil` (accepted while the property never held a table, `TypeError`
  after); everything else as the class of RGSS2 has it (`bitmaps` with nine slots,
  `map_data`, `flash_data`, `ox`, `oy`, `visible`, `viewport`, `update`, `dispose`).
- A write to a slot of `bitmaps` of a disposed tilemap is accepted and kept. The original
  ends with a page fault there; this is recorded as a known difference in
  `docs/compatibility.md`.
- Tests of the class in a game of RGSS3 over Ruby 1.9.2: the method surface, the values
  and errors of the measurement, and that what a script gives the class reaches the frame
  by the rules of RGSS3 (a hidden tilemap, a star tile of the lowest layer, a star tile of
  the third layer above a sprite, a shadow of the fourth layer, a map of three layers).
- Not in this change: anything of the drawing rules, which are in `vkxp-core` and tested
  there; `Viewport#dispose` (a tilemap whose viewport is disposed stays alive on the
  original); the wording of the count message of `Tilemap.new`; a new name for the field
  `passages` of the scene and of the plane op. XP and VX do not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-scene-classes`: the class `Tilemap` of RGSS3, its tables, what it draws, and a
  disposed one.

## Impact

- `crates/vkxp-binding/src/tilemap.rs`: the constructor and the definition by runtime, the
  property `flags`. `crates/vkxp-binding/src/lib.rs`: its documentation where it lists
  what a tilemap keeps, if a line names `passages` alone.
- New: `crates/vkxp-binding-rgss3/tests/tilemap_rgss3.rs`. A scenario on RGSS1 and RGSS2
  is one assertion in an existing test of `crates/vkxp-binding/tests/tilemap_rgss2.rs` and
  `scene.rs`, added and nothing changed.
- `docs/compatibility.md` (the section on VX Ace), `CHANGELOG.md`.
- Not touched: `vkxp-core`, `vkxp-proto` (no message and no version changes), the host
  crates, `crates/vkxp-binding/src/element.rs` and `rb.rs`, the allow-list
  `ruby192.allow`.
- Other changes in flight: none edits `tilemap.rs`.
  `run-rgss3-games-in-a-guest-of-their-own` and the other binding changes of RGSS3 write
  to the section on VX Ace of `docs/compatibility.md` and to `CHANGELOG.md`.
