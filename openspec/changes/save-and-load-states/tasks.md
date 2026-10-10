# Tasks

## 1. Protocol (`vkxp-proto`)

- [ ] 1.1 Add the state messages and raise the version to 8; fixtures and expected files that carry the version are regenerated. Verify: the scenarios of `specs/guest-host-protocol/spec.md`; `cargo test -p vkxp-proto -p vkxp-compose-ref --locked` passes.

## 2. Guest (`vkxp-core`, `vkxp-binding`)

- [ ] 2.1 `vkxp-core`: a way to mark every live bitmap and table as not yet sent. Verify: a test that extracts, marks and extracts again and gets every bitmap and table in full, and that applying the second packet alone to an empty compositor renders the frame of the first; `cargo test -p vkxp-core --locked` passes.
- [ ] 2.2 `vkxp-binding`: the copier trait in the run configuration, the pending save and the frame boundary rule, the resume sequence. Verify: the scenarios of `specs/rgss-runtime/spec.md` with a fake copier; `cargo test -p vkxp-binding --locked` passes.

## 3. Host (`vkxp-host`)

- [ ] 3.1 Slots, keys, notices, and the reset on a loaded state; the display trait learns to forget its resources (in `vkxp-vk` or `vkxp-compose-ref` only what that needs). Verify: the scenarios of "State slots and keys" and "Loading a state resets the host's side"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes.
- [ ] 3.2 The schedule of the headless runner. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes with a test of a scheduled request reaching the guest at its frame.

## 4. Processes (`vkxp`)

- [ ] 4.1 The copier (fork, new connection, `Attach`, park, resume, `_exit` on the end of the connection, signals) and the relay with several connections (slots, ending the running process, `Resume`, `Outcome`, ending every attached process at exit). Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.
- [ ] 4.2 `--states`, the default form by kind of run. Verify: scenarios "Thread form", "Malformed schedule" and "Headless default".
- [ ] 4.3 End-to-end tests for "Back to the saved frame", "Loading twice", "Empty slot", "Nothing left behind" and "Script error after a load". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 5. Documentation

- [ ] 5.1 README: the keys, `--states`, the limits of `design.md`, protocol version 8; `docs/decisions.md`: save states move from "reserved" to implemented, with the default form of the guest. Verify: both texts name F3 and F4.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate save-and-load-states --strict` passes.
