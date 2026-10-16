# Tasks

## 1. Required job and documents

- [ ] 1.1 Remove `continue-on-error` of `gate-aarch64` and update its comment and any test of `xtask` that expects it; `docs/compatibility.md`, `README.md`, `docs/decisions.md` (P1 row and "Goal and scope"), `CHANGELOG.md`. Verify: the scenarios of "aarch64 job"; `cargo test -p xtask --locked` passes.

## 2. Integration check

- [ ] 2.1 Run the gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate accept-arm-linux --strict` passes.
