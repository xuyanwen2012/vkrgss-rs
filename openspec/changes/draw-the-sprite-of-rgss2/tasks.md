# Tasks

## 1. Place, size and cut

- [x] 1.1 Rules 1, 2 and 4. Verify: the scenarios of "Sprite placement of RGSS2"; `cargo test -p vkxp-core --locked` passes.

## 2. Bush

- [x] 2.1 Rule 3 and `bush_opacity`. Verify: the scenarios of "Bush rows of RGSS2"; the same command.

## 3. Wave

- [x] 3.1 Rule 5 and the wave properties. Verify: the scenarios of "Wave of a sprite of RGSS2"; the same command.

## 4. Class and whole scenes

- [x] 4.1 The properties in RGSS2 only; the pending list. Verify: the scenarios of `specs/rgss-scene-classes/spec.md`; `cargo test -p vkxp-binding --locked` passes.
- [ ] 4.2 The held-out scenes in both compositors; `docs/compatibility.md`. Verify: the scenario "Random scenes"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-core -p vkxp-compose-ref -p vkxp-vk --locked` passes.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate draw-the-sprite-of-rgss2 --strict` passes.
