# Tasks

## 1. Turned sprites (`vkxp-core`)

- [ ] 1.1 Compute the box of a turned sprite in a viewport from the position less the viewport's origin and the source positions from the position itself. Verify: scenario "Viewport with an origin" on the cases of topics `turnedvp` and `geom2` in `ph-scene-geom-samples.txt`, copied into the crate's test data; `cargo test -p vkxp-core --locked` passes.
- [ ] 1.2 Make the lowest rows of the unclipped box the faint ones. Verify: scenario "Turned bush rows" and the cases of topic `bush`; the test at `extract/tests.rs` that states the old rule is rewritten; `cargo test -p vkxp-core --locked` passes.
- [ ] 1.3 Draw nothing of a turned sprite whose source rectangle leaves its bitmap. Verify: scenario "Source rectangle leaving the bitmap"; `cargo test -p vkxp-core --locked` passes.

## 2. Unrotated sprites (`vkxp-core`)

- [ ] 2.1 Draw a zoomed sprite with a bush depth as two stretches. Verify: scenario "Zoomed bush rows" and every row of the table of item 3b in `design.md` as a test; `cargo test -p vkxp-core --locked` passes.
- [ ] 2.2 Draw a stretched sprite whose source rectangle leaves its bitmap by the rule of item 5. Verify: scenario "Cut source rectangles" with every row of the table of measured columns in `design.md` as a test; `cargo test -p vkxp-core --locked` passes; `grep -n PLACEHOLDER crates/vkxp-core/src/turned.rs crates/vkxp-core/src/extract.rs crates/vkxp-core/src/extract/tests.rs` prints nothing for the notes this change settles.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-turned-bush-and-cut-sprites --strict` passes.
