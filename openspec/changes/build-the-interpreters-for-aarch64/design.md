# Design

## Context

- `crates/vkrgss-ruby-sys/build.rs`: `TARGETS` maps aarch64 to the x86_64 directory.
- `crates/vkrgss-ruby192-sys/build.rs`: `TARGET` is x86_64 only; other targets build empty
  and warn. `vkrgss-binding-rgss3`, `vkrgss-guest-main-rgss3` and `vkrgss-guest-rgss3` set
  `vkrgss_interpreter_absent` for any other target.
- Measured headers: `../../vkxp/target/p1-config/` (`config-187-aarch64.h`,
  `config-192-aarch64.h`, `onig-config-aarch64.h`, `REPORT.md`), read only. The flow has no
  aarch64 machine and no network.

## Decisions

- **Headers.** Copy each x86_64 header to `config/aarch64-unknown-linux-gnu/` and change only
  the platform strings, exactly as the measured headers have them (`aarch64-linux`). Keep the
  hand edits of the 1.9.2 header (`RUBY_EXEC_PREFIX`, no `STACK_END_ADDRESS`) and adapt its
  comment: written by the release's `configure` on a Jetson Orin Nano, Ubuntu 24.04.5, gcc
  13.3.0, with `--build=aarch64-unknown-linux-gnu` and the same `CFLAGS`, kept by hand with
  the same two edits. A test compares each aarch64 header with its x86_64 counterpart and
  allows only the platform strings (and the comment) to differ, so the two cannot drift.
- **Targets.** `vkrgss-ruby-sys`: aarch64 maps to its own directory. `vkrgss-ruby192-sys`:
  a list of two targets; the three dependent build scripts use the same list (one constant
  each, or read from one place if simpler).
- **What the flow can verify.** On x86_64: the gate. For aarch64: that
  `cargo check --target aarch64-unknown-linux-gnu` is not possible without the target's
  C library, so do not try; the owner's Jetson and the CI job `gate-aarch64` verify it after
  the merge. Say so in `design.md`.
- **Documents.** `docs/compatibility.md`: x86_64 supported; on aarch64 both interpreters are
  built and tested by the job `gate-aarch64`, and support follows when P1 is accepted.

## Out of scope

Making `gate-aarch64` required, comparing frames and saves between the architectures (the
acceptance of P1), other architectures.

## Follow-ups

None yet.
