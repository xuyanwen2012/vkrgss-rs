# Tasks

## 1. Draw order (`vkxp-core`)

- [x] 1.1 Give the stamp once at creation and never renew it. Verify: the scenarios of `specs/core-scene/spec.md` with every row of the unit-test table of item 4 in `design.md` as a test; `cargo test -p vkxp-core --locked` passes.

## 2. Transfers within one bitmap (`vkxp-core`)

- [x] 2.1 Make a transfer with the same source and destination read the live pixels. Verify: the scenarios of `specs/core-bitmap/spec.md`; the measured cases of `ph-scene-selfblt-samples.txt`, copied into the crate's test data with the calls of the probe written as a test, all equal; `cargo test -p vkxp-core --locked` passes.

## 3. Colour conversion and notes (`vkxp-core`)

- [ ] 3.1 Wrap negative red, green and blue in `Color::to_proto`. Verify: the scenarios of `specs/core-values/spec.md`; `cargo test -p vkxp-core --locked` passes.
- [ ] 3.2 Reword the confirmed notes named in `design.md`. Verify: `grep -n PLACEHOLDER crates/vkxp-core/src/geometry.rs crates/vkxp-core/src/color.rs crates/vkxp-core/src/scene.rs crates/vkxp-core/src/bitmap.rs` prints nothing and `sort` in `extract.rs` has no such note.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-draw-order-self-transfer-and-colour-wrap --strict` passes.
