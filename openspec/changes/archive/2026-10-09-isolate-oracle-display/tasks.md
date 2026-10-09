# Tasks

## 1. Display (`xtask`)

- [x] 1.1 Start the display server with `-displayfd` and read the number back with a timeout. Verify: tests for the two scenarios of `specs/oracle-harness/spec.md` with a stand-in server program, and an ignored test with the real `Xvfb` that starts two harness displays at once and connects to both.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate isolate-oracle-display --strict` passes.
