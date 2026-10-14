# Tasks

## 1. Built-in scripts

- [x] 1.1 Evaluate the built-in scripts from bytes; the test `builtin_encoding_rgss3.rs`. Verify: the scenarios of `specs/rgss-runtime/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-binding-rgss3 --locked --test builtin_encoding_rgss3` passes.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate evaluate-the-built-in-scripts-of-rgss3-as-bytes --strict` passes.
