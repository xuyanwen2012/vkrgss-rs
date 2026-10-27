# Tasks

## 1. Framing and the stream link

- [x] 1.1 The incremental reader in `framing.rs`, used by `GuestReceiver`; the non-waiting receive with `MSG_DONTWAIT` instead of switching the socket's mode; framing errors logged with their reason before `Disconnected`. Verify: tests of the scenarios "One byte at a time", "Oversized length" (incremental reader) and "Send while polling"; the existing tests of `framing.rs` and `link.rs` pass; `cargo test -p vkrgss-proto --locked` passes.

## 2. The relay

- [x] 2.1 Non-blocking reads and writes of the running guest with the outgoing buffer and the deadline checked on every turn; attaching without blocking, with the ordering rule kept; `accept` errors backed off; the bound of 256 MiB on messages that are not frame packets. Verify: relay tests of "Stopped inside a message", "Not reading", "Silent connection" and "Flood of titles" with stand-in processes; the existing relay tests pass; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss --locked` passes, including the states tests; `cargo clippy -p vkrgss-android --target aarch64-linux-android` (or the gate part that checks Android) passes.

## 3. The session

- [ ] 3.1 Quit deadline (`SessionEnd::GaveUp`, scheduled by `next_wake`), `run_windowed` without the join after giving up, the program's failure report; injected keys without the program's shortcuts; the headless clock moved only to ticks; `GuestForm`'s default; the `--guest-role` branch of `main.rs` if unused. Verify: session tests of "Guest does not shut down", "Program shortcuts", "Reset by an injected key" and the existing scenarios of both modified requirements; scenario "Script in an endless loop" as a test of the windowed driver with the test doubles `windowed.rs` already uses for the thread form, and through `vkrgss` with a guest process where a windowed run can be driven in tests (otherwise the process form is covered by the relay tests and the owner checks it by hand; say which in `design.md`); "Save and load" of the headless clock; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-host --locked` passes.

## 4. Documents

- [ ] 4.1 `docs/running.md` (closing a game whose scripts do not end, the host keys and injected keys), `docs/architecture.md` (the relay), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Checks

- [ ] 5.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate end-the-session-without-the-guest --strict` passes.
