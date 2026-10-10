# Tasks

## 1. Move (`vkxp-core`)

- [x] 1.1 Save the list of test names of the crate before any edit (under `local/`). Verify: the file exists.
- [x] 1.2 Move the test modules that the rule of `design.md` selects, one commit per module. Verify: the scenarios of `specs/workspace-structure/spec.md`; the saved list equals the list after the move; `git diff --stat` against the starting commit shows no file outside `crates/vkxp-core` and the change's own directory.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate move-core-tests-out-of-line --strict` passes.
