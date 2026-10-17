# Filter the system calls of the guest

## Why

The guest process confines itself with Landlock only (`crates/vkrgss-guest/src/sandbox.rs`);
the workspace has no seccomp code. `docs/decisions.md` speaks of "the guest's Landlock and
seccomp" (Linux packages) and of "a separate guest process under seccomp" (Android), which
is not true today. The Android measurement of 2026-10-16/17 found Landlock absent on both
test phones (ENOSYS) and a seccomp-BPF filter installable, enforced and inherited on both
kernels; on Android seccomp is the whole confinement besides the app's private directory.
On Linux it closes what Landlock does not cover (process tracing, other socket families,
loading kernel code).

## What Changes

- Before the first script runs, the guest installs a seccomp-BPF filter on Linux and
  Android (`PR_SET_NO_NEW_PRIVS`, `SECCOMP_SET_MODE_FILTER` with `TSYNC`), after the Landlock
  rules where they exist. The filter checks the architecture and returns `EPERM` for a
  deny-list of system calls a game never needs: `execve`, `execveat`, `ptrace`,
  `process_vm_readv`/`writev`, `socket` of every family but `AF_UNIX`, `bpf`,
  `perf_event_open`, `mount`/`umount2`/`pivot_root`/`chroot`, `init_module`/
  `finit_module`/`delete_module`, `kexec_load`/`kexec_file_load`, `keyctl`/`add_key`/
  `request_key`, `userfaultfd`, `unshare`/`setns`, `reboot`, `swapon`/`swapoff`, the `io_uring` calls, and
  `clone` with namespace flags; `clone3` returns `ENOSYS`.
- `--sandbox off` and `--allow-process-functions` leave the filter out, as they leave
  Landlock out today.
- The copies of the guest (save states) inherit the filter; nothing they do is denied.
- `docs/decisions.md` says what the confinement is per platform.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the confined guest also filters system calls.

## Impact

`crates/vkrgss-guest` (a new `seccomp.rs`, one call from where `confine` is called), its
tests and those of `vkrgss` for the sandbox, `docs/decisions.md`, `docs/running.md` if it
describes the sandbox, `CHANGELOG.md`.
