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

## Out of scope

An allow-list filter; macOS confinement; changing what Landlock allows.

## Follow-ups

None yet.
