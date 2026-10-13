# Tasks

## 1. Core

- [x] 1.1 A sprite, a window, a plane and a tilemap can change their viewport. Verify: the scenario "Seen by the scene" of "Moving an element to another viewport" at the level of the scene; `cargo test -p vkxp-core --locked` passes.

## 2. Classes of RGSS2

- [x] 2.1 The three writers and `Plane#viewport(v)`. Verify: the scenarios of "Moving an element to another viewport"; `cargo test -p vkxp-binding --locked` passes.
- [x] 2.2 `Sprite#width` and `height`, `Plane#visible`, the contents of a new window, no `Window#stretch`, no `Hangup`. Verify: the scenarios of "Small differences of RGSS2"; the same command passes.

## 3. Method list

- [ ] 3.1 The committed method list of RGSS2 and its test with the two lists of pending names. Verify: the scenarios of "Method surface of the scene classes"; the same command passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate move-elements-between-viewports-in-rgss2 --strict` passes.
