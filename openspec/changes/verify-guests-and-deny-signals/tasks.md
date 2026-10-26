# Tasks

## 1. Identity of attaching processes

- [x] 1.1 Peer credentials in `attached()` (Linux and Android `SO_PEERCRED`, macOS `LOCAL_PEERPID` and `getpeereid`), the refusals of `design.md`, the first connection compared with `child.id()`, `kill()` guarded and its SAFETY comment rewritten; adapt the relay's tests so that no test kills the test process. Verify: unit tests of `process.rs` for the scenarios "Stated number differs" (a number of another process, and 0) and "First connection from another process"; `cargo test -p vkrgss --locked --lib` passes; `cargo clippy -p vkrgss-android --target aarch64-linux-android` (or the gate part that checks Android) passes.

## 2. Signals of a confined guest

- [x] 2.1 Move the guest's own signals to allowed forms: `parent_watch.rs`, and the end of a script by the signal of an uncaught `SignalException` for both interpreters (default action, then `raise`). Verify: the tests of the requirement "Interrupt of a script" (`script-mode`) pass for `rgss1` and `rgss3`; the tests of `parent_watch` pass.
- [ ] 2.2 Add `kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and `pidfd_send_signal` to the denied calls; Landlock's signal scope where the ABI has it, with the report of what an older ABI leaves out. Verify: `vkrgss-guest/tests/seccomp.rs` checks every new number and the scenarios "Signal to another process" and "Own threads" in forked children (skipping with a printed reason where seccomp or Landlock cannot be used); the unit test of `missing()` covers ABI 5 and 6; scenario "Script ends by its signal" passes with the confinement on.

## 3. State answers and loads during a refill

- [ ] 3.1 The relay drops state answers and attachments from the running guest, and keeps a load of the refilling slot until its copy attaches. Verify: relay tests for "Guest sends a loaded answer" and "Load again at once"; the existing relay tests and `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss --locked` pass, including scenario "Copies still attach".

## 4. Documents

- [ ] 4.1 `docs/running.md` (section "Sandbox": signals, what Landlock below ABI 6 and Android leave, the identity check), `docs/decisions.md` (the sandbox paragraph), `README.md` ("Browser-grade isolation" says no more than holds), `CHANGELOG.md`; the macOS Follow-up stays in `design.md`. Verify: the tests of `xtask` that check the documents pass; no document says that a guest cannot signal while one can.

## 5. Checks

- [ ] 5.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate verify-guests-and-deny-signals --strict` passes.
