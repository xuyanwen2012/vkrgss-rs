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

## Implementation notes

- **One list for 1.9.2.** The targets of Ruby 1.9.2 are `TARGETS` in
  `crates/vkrgss-ruby192-sys/targets.rs`. The build script of the crate includes it as a
  module, the build scripts of `vkrgss-binding-rgss3`, `vkrgss-guest-main-rgss3` and
  `vkrgss-guest-rgss3` include it by `#[path]`, and `tests/targets.rs` tests it, with
  `riscv64gc-unknown-linux-gnu` as a target outside it. For a target outside the list the
  build script of `vkrgss-ruby192-sys` now sets `vkrgss_interpreter_absent` as the three
  others do, and its library and test files are empty under that `cfg` instead of under a
  `cfg` on `target_arch`. Its `Cargo.toml` cannot read the list: the `cfg` of its
  dependencies names x86_64 and aarch64 Linux with glibc.
- **Platform strings in tests.** The tests that expect `RUBY_PLATFORM` (`calling.rs` and
  `upstream.rs` of `vkrgss-ruby-sys`, `original.rs` and `upstream.rs` of
  `vkrgss-ruby192-sys`) expect `aarch64-linux` on aarch64, and the `rbconfig.rb` of the test
  library of 1.9.2 takes its architecture from `RUBY_PLATFORM`.
- **Outside the scope.** `crates/vkrgss-guest-rgss3/src/main.rs`: the line that the
  executable writes on a target without Ruby 1.9.2 named x86_64 as the only target; it now
  names both. Two lines, needed so that the scenario "Another target" of the guest stays
  true. `xtask/src/gate.rs`: the new test targets `config_headers` and `targets` of
  `vkrgss-ruby192-sys` are added to `RUBY192_TESTS`, as its documentation asks for a new
  test target; without them the test that compares the parts with the gate fails. Four
  lines.
- **aarch64 itself is verified after the merge.** The flow has no aarch64 machine and no
  C library of that target, so `cargo check --target aarch64-unknown-linux-gnu` was not
  tried. On x86_64 the gate passes, the header tests compare the headers of aarch64 with
  those of x86_64, and the `cfg` that a target outside the list gets was built and tested by
  hand (`RUSTFLAGS='--cfg vkrgss_interpreter_absent' cargo test -p vkrgss-ruby192-sys`, every
  test binary but `config_headers` and `targets` empty). That both interpreters build and
  pass their tests on aarch64 is verified by the CI job `gate-aarch64` and on the owner's
  Jetson after the merge.

## Follow-ups

- Specifications outside this change still name x86_64 as the only target of the second
  interpreter: the scenario "The crates of the second interpreter on another target" and
  the scenario on the test file of RGSS3 of `workspace-structure`, and the scenario of the
  configuration for another target of `binding-over-two-interpreters`. Their wording ("a
  target other than `x86_64-unknown-linux-gnu`") now includes aarch64, where the crates are
  no longer empty.
- `crates/vkrgss/tests/rgss3.rs` is compiled for x86_64 only (`cfg` on `target_arch`), so
  the end-to-end tests of RGSS3 do not run on aarch64 although the guest is built there.
- Documents outside the scope that still say Ruby 1.9.2 is for x86_64 alone or that Ruby
  1.8.7 borrows the header of x86_64: the row of `vkrgss-ruby192-sys` in
  `docs/architecture.md` and the item on the `config.h` of aarch64 under "Platforms" of
  `docs/decisions.md` (a settled document, changed only by a change that says so).
