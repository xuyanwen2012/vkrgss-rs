# Tasks

## 1. Session (`vkxp-host`)

- [ ] 1.1 Shift the transition steps by one frame. Verify: tests for the six scenarios of `specs/host-session/spec.md`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-transition-steps --strict` passes.
