# Tasks

## 1. Function

- [x] 1.1 `Graphics.resize_screen` in RGSS2 only; the size of the scene can change. Verify: the scenarios of "Resizing the screen" except the dump; `cargo test -p vkxp-core -p vkxp-binding --locked` passes.

## 2. Shown frames

- [x] 2.1 A run whose frames change their size is shown and dumped correctly, headless and on both compositors. Verify: the scenario "Dump after a resize"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host -p vkxp-vk -p vkxp --locked` passes.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate resize-the-screen-in-rgss2 --strict` passes.
