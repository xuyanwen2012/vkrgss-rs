# Tasks

## 1. Text (`vkxp-text`)

- [x] 1.1 Give fallback glyphs their own advance. Verify: the scenarios of `specs/text-rendering/spec.md`, the existing "Per-character fallback" scenarios, and the committed measured samples.

## 2. Win32API (`vkxp-binding`)

- [x] 2.1 Accept the three functions. Verify: a Ruby test for the scenario of `specs/rgss-runtime/spec.md`.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate keep-fallback-advances-and-key-shims --strict` passes.
