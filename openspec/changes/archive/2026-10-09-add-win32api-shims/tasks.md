# Tasks

## 1. Call path (`vkxp-binding`)

- [x] 1.1 Route `Win32API#call` through a table of emulated functions with integer and buffer arguments. Verify: Ruby tests that an emulated function receives and overwrites a buffer in place, that names match without regard to case and suffix, and that an unknown function still returns 0 with one report.

## 2. Functions (`vkxp-binding`)

- [x] 2.1 Implement the profile functions with the directory rule. Verify: Ruby tests for the four profile scenarios and the outside-path scenario of `specs/rgss-runtime/spec.md`.
- [x] 2.2 Implement text conversion, ticks, metrics, window queries, `ShowCursor` and `RtlMoveMemory`. Verify: Ruby tests for the remaining scenarios.
- [x] 2.3 Document the table in the crate documentation. Verify: `cargo doc -p vkxp-binding --no-deps` builds without warnings.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-win32api-shims --strict` passes.
