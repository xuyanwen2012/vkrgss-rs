# Design

## Context

- `crates/vkxp-ruby-sys/tests/common/mod.rs` builds Ruby's test executable and runs the
  suites as child processes; `tests/upstream.rs` (in the gate) and `tests/sanitized.rs`
  (opt-in) use it.
- `test_should_propagate_signaled` in `third_party/ruby-1.8.7/test/ruby/test_beginendblock.rb`
  fails with `<""> expected to be =~ </Interrupt$/>` when SIGINT is ignored in the process
  that started the tests. Measured: `env --ignore-signal=INT cargo test -p vkxp-ruby-sys
  --test upstream` fails, `env --default-signal=INT` passes.
- `unsafe` is allowed in `vkxp-ruby-sys`.

## Decisions

- The runner resets SIGINT to its default disposition in the child before it executes the
  test executable (`CommandExt::pre_exec` with `libc::signal(SIGINT, SIG_DFL)`, which is
  async-signal-safe). If other signals the suites send to themselves show the same failure
  when ignored (check SIGTERM, SIGHUP, SIGQUIT, SIGUSR1 with `env --ignore-signal`), reset
  those too; reset nothing that showed no failure.
- A test of the crate runs the affected upstream test file with SIGINT ignored in the
  harness itself (set in the test, restored afterwards, or through a child `cargo`-free
  invocation of the runner) and expects it to pass.
- Nothing in the vendored tree changes.
- Measured during implementation, each with `env --default-signal=INT --ignore-signal=<signal>
  cargo test -p vkxp-ruby-sys --locked --test upstream`: SIGTERM, SIGHUP, SIGQUIT, SIGUSR1,
  SIGUSR2, SIGALRM and SIGPIPE pass when ignored, so only SIGINT is reset.
- The new test is `tests/upstream_signals.rs`. It ignores SIGINT in its own process, runs
  `test/ruby/test_beginendblock.rb` through the runner and restores the disposition;
  `Suites::run_unit_file` became `pub(crate)` for it. Without the reset the test fails with
  the message quoted under "Context".

## Risks / Trade-offs

- [A test that sets a process-wide disposition disturbs tests running in parallel] → Keep it
  in its own integration test file, which is its own process.

## Follow-ups

- With SIGCHLD ignored, every test of `tests/upstream.rs` fails within a fraction of a second,
  before any suite runs: the kernel reaps the children itself and the harness cannot wait for
  the compiler it starts. That is a signal of the harness, not one the suites send to
  themselves, and no known caller ignores it, so nothing was built for it.
