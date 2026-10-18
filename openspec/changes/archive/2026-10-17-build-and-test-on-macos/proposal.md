# Build and test on macOS

## Why

P2 of `docs/decisions.md` ("Platforms", macOS). The feasibility measurement of 2026-10-16 on
the owner's MacBook Air (Apple M4, macOS 27.0.1, MoltenVK 1.4.2; report in
`vkxp/target/p2-macos/REPORT.md` of the workspace, experimental diff
`scratch-all-experiments.diff` beside it) showed that the port is small:

- all 277 tests of `vkrgss-vk` pass on MoltenVK, the differential tests against the CPU
  reference included, with portability enumeration and a `WAIT` on timestamp queries;
- both interpreters build and pass their suites with headers measured on the Mac and a few
  build changes;
- the rest of the workspace compiles once Landlock and `prctl(PR_SET_PDEATHSIG)` are kept
  to Linux; then 2773 tests pass, and the 16 that fail do so for the reasons listed below.

This change makes the workspace build and its tests pass on aarch64-apple-darwin, without
packaging (the `.app`, the loader inside it and signing come later) and without a windowed
run, which needs the owner at the Mac.

## What Changes

- `vkrgss-vk`: `VK_KHR_portability_enumeration` and its instance flag when the loader offers
  them, `VK_KHR_portability_subset` at device creation when the device lists it; timestamp
  queries read with `WAIT`.
- `vkrgss-guest`: Landlock only on Linux; on macOS the guest is not confined (recorded in
  `docs/decisions.md` as a decision of this change; a later change may add `sandbox_init`);
  the guest notices the death of its parent through kqueue `EVFILT_PROC`/`NOTE_EXIT` (or a
  pipe) instead of `PR_SET_PDEATHSIG`.
- Both interpreter crates: the target `aarch64-apple-darwin` with the headers measured on
  the Mac, reviewed; `_REENTRANT` where `lgamma_r` needs it, `sigaction` through libc instead
  of `SYS_rt_sigaction`, no `-lcrypt`/`-lrt`, test links without `--start-group`.
  `RUBY_PLATFORM` is `aarch64-darwin` for both.
- Ruby 1.8.7: `Math` raises `Errno::ERANGE` for a pole or an overflow on macOS as on Linux,
  where macOS's libm does not set `errno` (in the CORE-MATH shim).
- The headless default device is chosen per operating system (llvmpipe on Linux, the Apple
  GPU on macOS).
- Tests that depend on Linux (`/proc`, inotify, ELF, a case-sensitive file system, file names
  that are not UTF-8, GNU tar) are kept to Linux or adapted, each with the reason.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: the workspace builds and its tests pass on macOS (P2 not yet
  accepted).

## Impact

`crates/vkrgss-vk`, `crates/vkrgss-guest`, `crates/vkrgss-ruby-sys`,
`crates/vkrgss-ruby192-sys`, `crates/vkrgss-host` (headless default), the tests of
`vkrgss`, `vkrgss-vfs`, `vkrgss-binding-rgss3` and `xtask` where they depend on Linux,
`third_party/core-math` shim (errno), `docs/building.md`, `docs/compatibility.md`,
`docs/decisions.md`, `CHANGELOG.md`. No change to what a game sees on Linux.
