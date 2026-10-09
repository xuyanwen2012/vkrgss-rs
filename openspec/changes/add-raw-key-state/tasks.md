# Tasks

## 1. Protocol and input state (`vkxp-proto`, `vkxp-core`)

- [x] 1.1 Add raw keys and pointer to the snapshot, raise the version, and keep the previous snapshot in the input state. Verify: the scenarios of `specs/guest-host-protocol/spec.md` and the existing input tests.

## 2. Host (`vkxp-host`)

- [x] 2.1 Fill raw keys and pointer from window events and extend the recording format. Verify: tests for the three scenarios of `specs/host-session/spec.md` on the translation functions and the recorder.

## 3. Guest (`vkxp-binding`)

- [x] 3.1 Emulate the five functions. Verify: Ruby tests for the three scenarios of `specs/rgss-runtime/spec.md`.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-raw-key-state --strict` passes.
