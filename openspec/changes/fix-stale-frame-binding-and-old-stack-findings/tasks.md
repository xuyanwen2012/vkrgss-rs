# Tasks

## 1. The stale binding

- [x] 1.1 `forget_bound` and its call in `ensure_target`. Verify: `VK_LOADER_LAYERS_DISABLE='*validation*' VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes (it fails in two tests before the fix), and so does the same command without `VK_LOADER_LAYERS_DISABLE`.
- [x] 1.2 The gate step `test-unvalidated` and the texts that list the steps. Verify: the scenarios of `specs/workspace-structure/spec.md`; `cargo test -p xtask --locked` passes.

## 2. The older stack

- [x] 2.1 The decision not to release on a headless surface of Mesa older than 25.3.4, with its unit test. Verify: scenario "Decision by driver"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes.
- [x] 2.2 The harness drops the false report of layers below 1.4.313 and says so, with a unit test of the decision. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes.

## 3. Integration check

- [x] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-stale-frame-binding-and-old-stack-findings --strict` passes.
