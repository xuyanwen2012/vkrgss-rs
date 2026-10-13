# Tasks

## 1. Decisions

- [ ] 1.1 `docs/decisions.md`: the section "VX Ace", "Process", the rows A0 to A3 and the risks. Verify: the scenarios of `specs/supported-runtimes/spec.md`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate plan-vx-ace --strict` passes.
