# Tasks

## 1. Documents and notice

- [x] 1.1 `docs/compatibility.md`, `docs/running.md`, the README's table of runtimes and the warning of a run of RGSS3. Verify: the scenarios of `specs/supported-runtimes/spec.md`; `grep -rn "before its title" README.md docs crates/vkxp/src` finds nothing.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate say-what-runs-of-vx-ace --strict` passes.
