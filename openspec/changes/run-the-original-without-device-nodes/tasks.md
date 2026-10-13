# Tasks

## 1. Oracle

- [ ] 1.1 Wrap every Wine command in the sandbox, set the VA-API variables, add `bwrap` to the prerequisites; the unit tests. Verify: the scenarios of `specs/oracle-harness/spec.md`; `cargo test -p xtask --locked` passes.
- [ ] 1.2 `docs/development.md`: the prerequisite and why. Verify: the section on the original side names `bwrap`, `/dev/dri` and the network.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-the-original-without-device-nodes --strict` passes.
