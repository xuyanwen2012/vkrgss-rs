# Tasks

## 1. Input (`vkxp-core`)

- [x] 1.1 Replace the per-button repeat by the single repeating button. Verify: the scenarios of "Repeat" and the three tables under "Repeat" in `design.md` as tests; `cargo test -p vkxp-core --locked` passes.
- [x] 1.2 Replace the trigger-time comparison of `dir4` by the axis flag. Verify: the scenarios of "Direction queries" and the two step tables under "Directions" in `design.md` as tests; `cargo test -p vkxp-core --locked` passes.

## 2. Colour bounds (`vkxp-core`, tests of `vkxp-binding`)

- [x] 2.1 Clamp colour components as measured. Verify: the scenarios of `specs/core-values/spec.md` and `specs/rgss-values/spec.md`; `cargo test -p vkxp-core -p vkxp-binding --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.

## 3. Oracle probe and notes

- [x] 3.1 Make `xtask/src/oracle/probe.rb` answer `repeat?` and `dir4` by the same rules. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p xtask --locked` passes, with a test of the vkxp side that holds two buttons in a schedule.
- [x] 3.2 Reword the confirmed notes named in the proposal. Verify: `grep -n PLACEHOLDER crates/vkxp-core/src/rect.rs crates/vkxp-core/src/table.rs crates/vkxp-core/src/input.rs` prints nothing, and `crates/vkxp-core/src/element.rs` has no `PLACEHOLDER` at the window's `active` and `pause` setters or at the tilemap's update.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-input-rules-and-colour-bounds --strict` passes.
