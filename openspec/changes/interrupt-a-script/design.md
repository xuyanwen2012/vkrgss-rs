# Design

## Context

- `docs/decisions.md`: the interpreter runs only on the script thread; host signal handlers are
  restored after `ruby_init`. Script mode reuses this start-up, so Ruby's handler of SIGINT is
  dropped and the default action ends the process.
- The follow-up "Interrupt" of `run-a-ruby-script` (its `design.md`) describes the gap.

## Decisions

- **Keep Ruby's handler of SIGINT in script mode.** The start-up of an interpreter takes an
  option (or script mode its own entry) that leaves Ruby's disposition of SIGINT in place;
  every other signal is restored as now, and games pass nothing new.
- **Delivery to the interpreter.** Ruby 1.8.7 handles a signal on whatever thread it arrives on
  through its own pending-signal flag; Ruby 1.9.2 needs it handled by its timer thread and the
  main thread of the VM. A process with other threads (the Rust main thread, if the script runs
  on a thread of its own) blocks SIGINT on those threads, so the kernel delivers it to the
  interpreter's thread. Whatever is done, verify it with a signal sent to the process, not to a
  thread.
- **The end.** Uncaught `Interrupt` ends as `ruby` does: report, `at_exit`, then the process
  ends by SIGINT (default action re-raised), not by `exit(1)`.
- **Tests.** Integration tests that start a script which prints a line and sleeps, wait for the
  line, send SIGINT to the process, and check the output of `ensure`/`at_exit`/`rescue
  Interrupt` and the end by SIGINT, for both interpreters. Bounded by a timeout.

## Out of scope

SIGTERM and other signals; signals in games.

## Follow-ups
