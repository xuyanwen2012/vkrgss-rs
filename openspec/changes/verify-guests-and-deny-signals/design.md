# Design

## Context

- `docs/decisions.md`, the sandbox paragraph: Landlock (Linux) and a seccomp deny-list (Linux
  and Android) confine a guest process; macOS has neither. The host is not restricted.
- `crates/vkrgss/src/process.rs`: `attached()` reads the version statement and the first
  message of a connection and keeps the stated `pid`; `Attached::end`, `kill`, `Relay::load`
  and `Relay::finish` send SIGKILL to it. `GuestProcess::start` never compares the first one
  with `child.id()`. `vkrgss-android` compiles this file by `#[path]`.
- `crates/vkrgss-guest/src/seccomp.rs`: `DENIED` (28 numbers) has no signal call; the filter is
  installed with `SECCOMP_FILTER_FLAG_TSYNC` before the first script and binds the copies.
- `crates/vkrgss-guest/src/sandbox.rs`: Landlock rules with `landlock` 0.4 (which knows ABI 6
  and its scopes); the rules bind the interpreter thread and what it starts, the copies
  included. The guest's main thread is not confined and runs no script.
- Signals the guest sends itself today: `parent_watch.rs` (`kill(getpid(), SIGKILL)`); Ruby
  1.8.7's `ruby_default_signal` (`kill(getpid(), sig)`), which the binding calls to end a
  script by the signal of an uncaught `SignalException`; Ruby 1.9.2's timer thread
  (`pthread_kill`, that is `tgkill`) and its `ruby_default_signal` (`raise`, also `tgkill`);
  the C library's `abort` and `raise` (`tgkill`).

## Decisions

- **Identity from the kernel.** `attached()` reads the peer's credentials before it reads
  anything: `SO_PEERCRED` (`struct ucred`: pid, uid) on Linux and Android, `LOCAL_PEERPID` and
  `getpeereid` on macOS, through `libc` (no new dependency). A connection is refused, with one
  warning naming the reason, when the peer's user is not the program's effective user, when
  its process number is not positive, or when the `pid` of its `Attach` differs from the
  kernel's. The number kept in `Attached` is the kernel's. `GuestProcess::start` also refuses
  a first connection whose number is not `child.id()`. The credentials are those of the
  process that connected; copies connect themselves (`copier.rs`), so they match.
- **Signal only verified numbers.** `kill()` keeps its form but is only reachable with a number
  that came from the kernel; it also refuses a number that is not positive, as a second guard.
  Its SAFETY comment says where the number comes from.
- **Tests that stand in for a guest.** The relay's tests start `sleep` as the "guest" and
  connect from the test process. With the check, either the stand-in process itself connects
  (a small helper, e.g. the test binary re-executed or `socat`-free Rust in a forked child),
  or the tests keep connecting from the test process and state the test process's own number,
  while the kill target in those tests becomes a forked child of the test that connected. The
  implementer picks one; no test may kill the test process.
- **Signals of a confined guest, Landlock.** Where the kernel has Landlock ABI 6 or later, the
  ruleset also scopes signals (`Scope::Signal`): the interpreter thread and what it starts,
  copies included, can signal nothing outside the Landlock domain. The report of the
  best-effort mode names what an older ABI leaves out ("signals to other processes" below
  ABI 6). Verify on the running kernel that signals between threads of the guest (Ruby 1.9.2's
  timer thread to the main thread of the VM, which is in the domain) still arrive; if the
  kernel refuses them, record it and keep only the seccomp rule.
- **Signals of a confined guest, seccomp.** On every Linux and Android kernel the filter answers
  `EPERM` to `kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and `pidfd_send_signal`.
  `tgkill` stays allowed: the C library's `raise`, `abort` and `pthread_kill` use it, and its
  target cannot be compared with the caller in a static filter that the copies inherit (each
  copy has another process number). The guest's own uses move to allowed forms:
  `parent_watch.rs` ends the process with `raise(SIGKILL)` (or `_exit`, if that keeps the
  documented outcome), and the binding ends a script after an uncaught `SignalException` by
  setting the default action and calling `raise`, for both interpreters, instead of Ruby
  1.8.7's `ruby_default_signal` (no vendored patch). With Landlock below ABI 6 (Android, older
  kernels), a process that already runs native code of its own could still reach another
  process with `tgkill`; scripts cannot, because no Ruby function reaches `tgkill`. The
  documents say so.
- **State answers come from the relay.** `Relay::guest_message` drops `StateSaved`,
  `StateLoaded`, `StateFailed` and `Attach` from the running guest, with one warning per kind
  and run; the relay alone makes these answers. The session is not changed: in process form
  only the relay talks to it, and in thread form there are no states.
  One answer was the guest's to give until now: a guest that cannot copy itself (with
  `--allow-process-functions`, or when `fork` fails) sends `StateFailed { load: false }` for
  a request to save, and the scenario "a save is answered with the failure" depends on it.
  The relay keeps the slots of the requests to save it has handed on and not seen a copy
  for; a failed save for one of them is the guest's report that it could not copy, and the
  relay answers that request itself, once. Any other failed answer is dropped like the rest.
- **A load during a refill.** When `LoadState` names the slot in `refilling`, the relay keeps
  the request and acts on it when the new copy of that slot attaches; if the resumed guest
  ends, or sends a message before a copy of it attached, the kept request is answered
  `StateFailed { load: true }` as today. A second request while one is kept replaces it.
- **macOS.** The credential branch for macOS cannot be built here (the C interpreters need the
  SDK). It is written against `libc`'s macOS constants and checked by the owner's session at
  the Mac at the merge (a Follow-up records that). Signals on macOS are left to the stub of
  `Process.kill` of the change `confine-loading-and-process-functions`.

## Out of scope

The relay's blocking reads and writes, the accept loop and budgets (change
`end-the-session-without-the-guest`); the stubs of the process functions in Ruby (change
`confine-loading-and-process-functions`); a sandbox for macOS.

## Follow-ups

- The macOS branch of the credentials is checked at the Mac at the merge.
