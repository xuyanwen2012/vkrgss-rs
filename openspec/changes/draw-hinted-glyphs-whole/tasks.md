# Tasks

## 1. Tests first

- [x] 1.1 The tests of the three scenarios, failing before the fix where the cause shows on the test fonts (record in design.md which fail). Verify: `cargo test -p vkxp-text --locked` runs them.

## 2. The fix

- [ ] 2.1 Draw the hinted outline whole. Verify: the scenarios of `specs/text-rendering/spec.md`; every test of `crates/vkxp-text` passes with its expected values unedited, `tests/rgss3_metrics.rs` included.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-hinted-glyphs-whole --strict` passes.
