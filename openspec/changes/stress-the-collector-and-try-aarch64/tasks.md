# Tasks

## 1. The switch

- [x] 1.1 `VKXP_GC_STRESS` as in `design.md`: the patch to `gc.c` with its entry in `PATCHES.md`, the reading at start-up, tests for the scenarios "Every allocation" and "Off". Verify: `cargo test -p vkxp-ruby-sys --locked` passes, including the vendored-source checks.

## 2. The command and the first run

- [x] 2.1 The stressed test beside `sanitized` and `cargo xtask stress` with its three steps, the constants for `n` and the skip list. Verify: `cargo test -p xtask --locked` passes; scenario "A lost value shows".
- [x] 2.2 Run `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress`, fix or list what it finds as `design.md` says, and fill "Checked". Verify: scenario "Stressed run"; every fix in the vendored tree has its entry in `PATCHES.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-ruby-sys -p vkxp-binding -p vkxp --locked` passes without the switch.

## 3. aarch64 job and documents

- [x] 3.1 The job `gate-aarch64`; `docs/compatibility.md`, `docs/building.md` and `docs/development.md` (the switch, the command, the job). Verify: the scenarios of `specs/workspace-structure/spec.md`; the tests of `xtask` that check the workflows and the links of the documents pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate stress-the-collector-and-try-aarch64 --strict` passes.
