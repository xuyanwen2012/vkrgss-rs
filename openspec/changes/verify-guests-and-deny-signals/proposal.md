# Verify guests and deny signals

## Why

The review of 2026-10-26 (`target/review-2026-10-26/merged.md`, findings B1, B2, F7 and Fable's
L8) found that the boundary between a guest process and the program is weaker than the
documents say:

- The relay sends SIGKILL to the process number a guest *states* in its `Attach` message
  (`crates/vkrgss/src/process.rs` `attached`, `Attached::end`, `kill`, `Relay::load`). Nothing
  compares it with the child that was started or with the process at the other end of the
  socket. A stated `pid: 0` makes `kill(0, SIGKILL)` end the program's own process group; a
  stated number of another process of the user ends that process. The SAFETY comment of
  `kill` ("the caller names a process of this run") is not upheld.
- The seccomp filter of the guest denies no system call that sends a signal, and Landlock as
  configured does not scope signals. A game script's `Process.kill(9, Process.ppid)` ends the
  window, and any process of the user can be signalled.
- The relay hands on every message of the guest but `Outcome`, so a guest can send
  `StateSaved`, `StateLoaded` or `StateFailed` itself; the session acts on them (forgets every
  resource, stops the audio, drops the pending tick), although the relay is documented as the
  only maker of these answers (`vkrgss-proto/src/message.rs`).
- A second load of a slot whose copy was just resumed, before the resumed guest has left its
  new copy, answers that the slot holds no state although it does.

## What Changes

- The relay takes the process number of every connection from the kernel (`SO_PEERCRED` on
  Linux and Android, `LOCAL_PEERPID` on macOS), refuses a connection whose stated number
  differs, whose user is another, or whose number is not positive, and requires the first
  connection to be the child it started. Only numbers taken this way are ever signalled.
- A confined guest process can no longer signal processes outside itself: Landlock's signal
  scope where the kernel has it (ABI 6, Linux 6.12), and on every Linux and Android kernel a
  seccomp rule that denies `kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and
  `pidfd_send_signal`. The guest's own code that signals itself moves to forms the filter
  allows.
- The relay drops `StateSaved`, `StateLoaded`, `StateFailed` and `Attach` that come from a
  running guest, and says so once.
- A load of a slot whose copy is being replaced waits for the new copy instead of failing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: identity of attaching processes, signals of a confined guest, state answers
  made by the program alone, a load during a refill.

## Impact

`crates/vkrgss/src/process.rs` and its tests (also compiled into `vkrgss-android` by
`#[path]`); `crates/vkrgss-guest` (`seccomp.rs`, `sandbox.rs`, `parent_watch.rs`, any other
own-process signal, `tests/seccomp.rs`); `docs/running.md` (section "Sandbox"),
`docs/decisions.md` (the sandbox paragraph), `README.md` ("Browser-grade isolation"),
`CHANGELOG.md`. No protocol change, no picture changes.
