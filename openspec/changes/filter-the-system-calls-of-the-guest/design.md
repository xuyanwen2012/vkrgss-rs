# Design

## Context

- `sandbox.rs::confine(game_dir)` is called once on the interpreter's thread before the
  first script. Another change (`build-and-test-on-macos`, running on the Mac) makes
  Landlock Linux-only and touches `sandbox.rs`, `copier.rs`, `signals.rs` and
  `Cargo.toml` of `vkrgss-guest`: keep the seccomp code in a new file `seccomp.rs` and the
  edit of existing files to the few lines that call it, so the two merge cleanly.
- The Android measurement's probe (`../../vkxp/target/p3-android/probe/seccomp_probe.c`,
  read only) shows a working filter with `TSYNC` on both phones.

## Decisions

- **Deny-list, not allow-list.** An allow-list would have to follow every libc and every
  version of the interpreters, the audio and font code that run in the guest; a deny-list of
  calls no game needs is robust and still removes the dangerous ones. `EPERM` (not kill), so
  a script that tries `system` or `exec` gets `Errno::EPERM` as with Landlock's execute rule.
- **BPF by hand or crate.** Use the `libc` constants and a small hand-written BPF program
  (architecture check for `AUDIT_ARCH_X86_64` / `AUDIT_ARCH_AARCH64`, then the comparisons),
  or a well-known crate (`seccompiler`) if it adds no C dependency; say which and why. x32
  system calls are denied on x86_64 (`__X32_SYSCALL_BIT`).
- **`socket`.** Compare the first argument with `AF_UNIX`; `socketpair` stays allowed.
- **Order.** Landlock first (where available), then the filter, both before the first
  script; `NO_NEW_PRIVS` is set once. A failure to install the filter is reported once, like
  Landlock's, and the guest goes on.
- **Tests.** A test that starts a child under `confine` (or the filter alone) and checks
  that `execve` and `socket(AF_INET, ...)` fail with `EPERM` while `socket(AF_UNIX, ...)`,
  file I/O, threads and `fork` work; the scenario "States" of "Confined guest process"
  still passes; an end-to-end test where a script's `system("true")` raises `Errno::EPERM`.
  On Android, `cargo xtask android-test` of `vkrgss-guest` runs the same test on a phone if
  that command covers the crate; otherwise note it.

## Implementation

- **By hand.** The filter is a classic BPF program written in `crates/vkrgss-guest/src/seccomp.rs`
  with the structures and seccomp constants of `libc`; the BPF opcodes and the two
  `AUDIT_ARCH_*` values are written out, because `libc` gives them for Linux but not for
  Android, and so is `kexec_file_load` on aarch64 (294), which `libc` lacks for Android.
  `seccompiler` would add a dependency and a rule model for 25 numbers and one comparison of
  an argument, and with no network it could not be added to the lock file anyway.
- **Program.** The architecture is checked first; a system call of another architecture (on
  x86_64 the i386 ABI) and, on x86_64, one with `__X32_SYSCALL_BIT` gets `EPERM` too, not a
  kill. Then each number of `seccomp::DENIED` returns `EPERM`; `socket` loads the lower 32
  bits of its first argument (the kernel reads the family as an `int`) and returns `EPERM`
  unless it is `AF_UNIX`; everything else is allowed.
- **Call.** `play_in_process` calls `seccomp::filter()` right after `sandbox::confine`, inside
  the same `if`, so `--sandbox off`, `--allow-process-functions` and a guest on a thread
  leave both out; the edits of existing files are that call, the `mod` line and two lines of
  documentation in `lib.rs`. The `landlock` crate sets `PR_SET_NO_NEW_PRIVS` in
  `restrict_self`; the filter sets it only when `PR_GET_NO_NEW_PRIVS` says it is not set. A
  success is reported at the info level ("the guest process filters its system calls with
  seccomp"), a failure once as a warning, and the guest goes on.
- **Tests.** `crates/vkrgss-guest/tests/seccomp.rs` starts its own binary again with one test,
  which installs the filter and checks `execve` (through `Command`), every call of the list
  with -1 as every argument, an x32 call, `AF_INET`, `AF_INET6`, `AF_NETLINK` and
  `AF_PACKET`, and that a Unix socket by its path, a datagram socket, `socketpair`, file I/O,
  a thread and `fork` work, and that the forked copy is still filtered. In
  `crates/vkrgss/tests/end_to_end.rs`, `a_confined_guest_cannot_start_a_program` runs a game
  of RGSS1 with `--guest process`; the test of the scenario "States" also checks that the
  confined run reports the filter and the free one does not.
- **The script's `system("true")`.** In a confined guest `system` is the stub of the
  requirement "Process functions are off by default": it starts nothing and returns `nil`,
  and with `--allow-process-functions`, which turns the stub off, the guest is not confined.
  A script's `system` therefore never reaches `execve` under the filter, and Ruby's `system`
  returns `nil` or `false` when `exec` fails rather than raising. The end-to-end test checks
  that `system("true")` gives `nil` and reaches `execve` through `Process.exec("true")` of
  Ruby 1.8.7, which the stubs do not cover: it raises `Errno::EPERM` (the filter answers
  before Landlock, whose answer would be `EACCES`). See the first follow-up.

## Out of scope

An allow-list filter; macOS confinement; changing what Landlock allows.

## Follow-ups

- **Open question: the wording of the scenario "Denied system calls".** It says that a
  script's `system("true")` fails with `EPERM`, which cannot happen (see "The script's
  `system("true")`" above). Suggested wording: a script's `system("true")` starts nothing
  and `Process.exec("true")` raises `Errno::EPERM`. Until the owner decides, task 1.1 stays
  unticked.
- `io_uring` can create sockets (`IORING_OP_SOCKET`) without the system call `socket`, and
  `clone`/`clone3` with namespace flags do what `unshare` does. A script cannot reach either
  (`syscall` is turned off and no FFI is linked); denying `io_uring_setup` and the namespace
  flags of `clone` would be new entries of the list.
- `Process.exec` of Ruby 1.8.7 (and `Kernel#open("|...")` there) is not turned off by the
  stubs; under the sandbox the filter and Landlock stop it.
- On Android the crash handler of bionic starts `crash_dump` with `execve`, which the filter
  denies, so a crash of the guest leaves no tombstone.
