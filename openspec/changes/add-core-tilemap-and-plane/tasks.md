# Tasks

## 1. Shared tables and planes (`vkxp-core`)

- [x] 1.1 Add `SharedTable` (shared handle to a `Table` with a revision counter incremented by every write) and store such handles in tilemaps for map data, priorities and flash data. Verify: tests that writes through one handle are visible through another and bump the revision, and that reading does not.
- [x] 1.2 Draw planes as tiled quads with the shared linear transform, origin, zoom, opacity, blend type, colour, tone and viewport clip. Verify: tests for the six plane scenarios in `specs/core-scene/spec.md`, rendered with the reference compositor.
- [x] 1.3 Add the cached pre-tiled bitmap for planes that would exceed 64 repeats. Verify: a plane with a 4 by 4 bitmap over the full screen renders the same pixels as the formula of the tiling scenario, emits no more than 64 quads, and changing a pixel of the source bitmap changes the next frame.

## 2. Tiles (`vkxp-core`)

- [ ] 2.1 Add the autotile table as data, with a comment naming its source, and the functions that turn a tile identifier into source rectangles (tileset tile; autotile quarters for a given frame; single-tile sheets; nothing for identifiers below 48 or outside the bitmap). Verify: tests for the tile-identifier scenarios and the four autotile-pattern scenarios, including the test that draws all 48 patterns from a sheet with 48 distinct quarters.
- [ ] 2.2 Implement the tilemap cache: the visible cell range with wrap-around, `ground` and the per-priority bitmaps drawn with `blt`, and the invalidation rules of `design.md`. Verify: tests for the four tilemap-layout scenarios and the missing-tileset scenario.
- [ ] 2.3 Emit the ground quad and the per-row quads with their z values. Verify: tests for the four tile-priority scenarios.
- [ ] 2.4 Implement tilemap update and autotile animation. Verify: tests for the two animation scenarios.
- [ ] 2.5 React to table and bitmap changes. Verify: the shared-table scenario; a test that drawing into the tileset bitmap changes the next frame; the flash-data scenario.

## 3. Housekeeping (`vkxp-core`, `docs`)

- [ ] 3.1 Dispose cached bitmaps with their tilemap or plane, and stop caching for invisible ones. Verify: the arena's bitmap count returns to its earlier value after disposal.
- [ ] 3.2 Extend the random-scene property test with planes and tilemaps (random tables, origins, priorities, autotile sheets). Verify: the reference compositor renders every extracted packet without error.
- [ ] 3.3 Measure a rebuild: a 21 by 16 cell area with three full layers and autotiles. Verify: the time of one rebuild in a release-profile test is recorded in the commit message.
- [ ] 3.4 Add the paragraph to `docs/decisions.md` (section "Compositor") stating that tilemaps are currently composed on the CPU into cached bitmaps and that the tilemap op remains the target, and document the new modules in the crate documentation. Verify: the paragraph names this change.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-core-tilemap-and-plane --strict` passes.
