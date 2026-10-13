# Tasks

## 1. Brightness, waits and fades

- [x] 1.1 `Graphics.brightness`, `brightness=`, `wait`, `fadeout`, `fadein` in RGSS2 only. Verify: the scenarios of "Brightness of RGSS2" and "Waits and fades of RGSS2"; `cargo test -p vkxp-binding --locked` passes.

## 2. Transition

- [x] 2.1 `Graphics.transition` by runtime. Verify: the scenarios of "Freeze and transition"; the same test command passes.

## 3. Snapshot

- [x] 3.1 `Graphics.snap_to_bitmap` in RGSS2 only. Verify: the scenarios of "Snapshot of the scene"; `cargo test -p vkxp-core -p vkxp-binding --locked` passes.

## 4. End to end and documentation

- [x] 4.1 A generated game of RGSS2 run by the program, headless, with dumped frames: a fade and a transition after a freeze. Verify: the scenario "Dumped frames of a game"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.
- [x] 4.2 `docs/compatibility.md`: what of `Graphics` works for VX. Verify: the text names nothing that the code does not do.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-graphics-of-rgss2 --strict` passes.
