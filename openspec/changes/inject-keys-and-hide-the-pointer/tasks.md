# Tasks

## 1. Protocol

- [x] 1.1 The two messages, version 10, fixtures and expected files that carry the version regenerated, every `match` on the messages completed. Verify: the scenarios of `specs/guest-host-protocol/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-proto -p vkxp-compose-ref --locked` passes and the workspace builds.

## 2. Guest

- [ ] 2.1 `keybd_event`, `mouse_event`, `ShowCursor` send their messages; the toggle bits and `SetKeyboardState` as in `design.md`. Verify: the scenarios of `specs/rgss-runtime/spec.md`; `cargo test -p vkxp-binding --locked` passes.

## 3. Host

- [ ] 3.1 The session's injected keys, with replay, load and reset. Verify: scenarios "Raw key and button", "Release", "Full screen" and "Replay"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes.
- [ ] 3.2 Pointer visibility through the display to the window; the guest sends its value after a loaded answer. Verify: scenario "Hidden"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host -p vkxp --locked` passes.

## 4. Documentation

- [ ] 4.1 `docs/compatibility.md` (the paragraph on the shims says what is left: lock keys start off, movement by `mouse_event`, the one-update delay), `docs/architecture.md` and `README.md` (version 10 and what it added). Verify: `grep -rn PLACEHOLDER crates/vkxp-binding/src/win32api.rs` shows no line that says nothing is done; the documents name version 10.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate inject-keys-and-hide-the-pointer --strict` passes.
