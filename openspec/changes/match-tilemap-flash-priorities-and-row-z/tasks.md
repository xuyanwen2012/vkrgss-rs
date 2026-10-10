# Tasks

## 1. Priorities and row z (`vkxp-core`)

- [x] 1.1 Do not draw tiles whose priority is not 0 to 5, and leave the viewport's origin out of the z of a priority row. Verify: the scenarios of "Tile priorities" in `specs/core-scene/spec.md` and the z table of item 4 in `design.md` as tests; `cargo test -p vkxp-core --locked` passes.

## 2. Flash (`vkxp-core`)

- [x] 2.1 Replace the pulse, skip negative entries and show the overlay from the start. Verify: scenarios "Pulse" and "Measured samples"; `cargo test -p vkxp-core --locked` passes.
- [x] 2.2 Index a flash table of another size as measured. Verify: scenario "Flash table of another size" and the two rows for a flash table of 9 by 6 in `design.md`; `cargo test -p vkxp-core --locked` passes.

## 3. Notes

- [ ] 3.1 Reword the settled and confirmed notes. Verify: `grep -n PLACEHOLDER crates/vkxp-core/src/tilemap.rs crates/vkxp-proto/src/tiles.rs` prints nothing.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-tilemap-flash-priorities-and-row-z --strict` passes.
