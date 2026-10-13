# Tasks

## 1. Scene

- [x] 1.1 The order of elements by runtime. Verify: the scenarios of "Draw order of RGSS2"; `cargo test -p vkxp-core --locked` passes.
- [x] 1.2 Viewport flash and plane tile size by runtime. Verify: the scenarios of "Viewport flash of RGSS2" and "Plane tile size of RGSS2"; the same command.

## 2. Input

- [x] 2.1 Repeat timing by runtime in the core; the key map of VX in the host, chosen by the program. Verify: the scenarios of `specs/core-input/spec.md` and of "Keys of VX"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-core -p vkxp-host -p vkxp --locked` passes.

## 3. Modules

- [x] 3.1 Default vagueness, the provenance of the mask threshold, `blend_type=`. Verify: the scenarios of `specs/rgss-modules/spec.md` and "Mask thresholds of RGSS2"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-binding -p vkxp-host --locked` passes.

## 4. Documentation

- [x] 4.1 `docs/running.md`: the keys of a game of VX. Verify: the table equals the key map in the code.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate order-elements-and-read-input-as-rgss2 --strict` passes.
