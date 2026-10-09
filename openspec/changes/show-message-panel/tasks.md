# Tasks

## 1. Host (`vkxp-host`)

- [x] 1.1 Keep modal messages pending in windowed sessions until a confirm or cancel press or a click, and suppress that press. Verify: tests for the three scenarios of `specs/host-session/spec.md`.

## 2. Guest (`vkxp-binding`)

- [x] 2.1 Draw the panel and keep sending frames while waiting. Verify: Ruby tests for the three scenarios of `specs/rgss-runtime/spec.md`, with pixels read through the test helper.

## 3. Executable (`vkxp`)

- [ ] 3.1 Add `--log-p` and its configuration key. Verify: argument and configuration tests.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate show-message-panel --strict` passes.
