# Tasks

## 1. Session (`vkxp-host`)

- [x] 1.1 Apply the two transition cases and test the double freeze. Verify: tests for the scenarios of the modified requirement in `specs/host-session/spec.md`.

## 2. Headless display (`vkxp-host`)

- [x] 2.1 Make headless dumps the displayed image. Verify: tests on llvmpipe for the three scenarios of the dump requirement, and an end-to-end run of the `vkxp` executable on a generated game that freezes white, draws black, calls `Graphics.transition(4)` and dumps every frame: the dumps are white, three quarters, half and one quarter white, then black.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate dump-the-displayed-frame --strict` passes.
