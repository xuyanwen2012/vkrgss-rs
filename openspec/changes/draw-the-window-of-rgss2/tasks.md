# Tasks

## 1. Window of RGSS2 in the scene

- [x] 1.1 Skin layout, background and frame bitmaps, their place and alpha on the screen. Verify: the scenarios of "Background and frame of a window of RGSS2"; `cargo test -p vkxp-core --locked` passes.
- [x] 1.2 Openness. Verify: the scenarios of "Openness of a window of RGSS2"; the same command.
- [x] 1.3 Contents, cursor with its blink, pause sign, arrows. Verify: the scenarios of "Contents and cursor of a window of RGSS2" and "Pause sign and arrows of a window of RGSS2"; the same command.
- [x] 1.4 The latched state. Verify: the scenarios of "Latched state of a window of RGSS2"; the same command.

## 2. Class

- [ ] 2.1 `Window#openness` and `openness=`; the window of a game of RGSS2 is the window of RGSS2; the pending list. Verify: the scenarios of `specs/rgss-scene-classes/spec.md`; `cargo test -p vkxp-binding --locked` passes.

## 3. Whole scenes

- [ ] 3.1 The fixtures of the random scenes and of one case per rule, compared in the CPU reference and in the Vulkan backend. Verify: the scenario "Random scenes"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-core -p vkxp-compose-ref -p vkxp-vk --locked` passes.
- [ ] 3.2 `docs/compatibility.md`: the window of VX, and what is not measured. Verify: the text names nothing that the code does not do.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-the-window-of-rgss2 --strict` passes.
