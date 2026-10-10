# Tasks

## 1. Measuring (`vkxp-binding`, `vkxp`)

- [ ] 1.1 Add the option, the measuring and the rows. Verify: the scenarios of `specs/host-session/spec.md`.

## 2. Documents

- [ ] 2.1 Describe the rows in `README.md`. Verify: the README names the four rows.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate split-guest-frame-time --strict` passes.
