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
- **As built.** `Options` of both interpreter crates has `interrupt`, false by default and in
  `Options::for_rgss`, so games pass nothing new; `run_script` of `vkrgss-guest-main` sets it
  and blocks SIGINT on its own thread once the interpreter has started
  (`signals::block_interrupt` of `vkrgss-guest`). With it Ruby 1.8.7 keeps its handler of
  SIGINT. Ruby 1.9.2 keeps its handlers of SIGINT and SIGVTALRM and starts its timer thread
  in `ruby_init`, as the release does (`ruby_vkxp_lazy_timer_thread` stays 0): the timer
  thread hands the signal to the main thread, and wakes a main thread that blocks in a read
  with SIGVTALRM, whose default action would end the process. The interpreter thread's mask
  is put back as for a game. Once Ruby is finalised every disposition is put back. The end
  by SIGINT was already there: `run_script` of the binding ends the process by the signal
  of an uncaught `SignalException`.
- **Tests as built.** The test sends SIGINT 200 ms after the line, because Ruby 1.8.7, as
  `ruby` 1.8.7, takes a signal that comes just before a call that blocks only when the call
  returns; the scripts sleep 20 s, so the tests pass either way. A test also interrupts a
  read of the standard input over Ruby 1.9.2.

## Out of scope

SIGTERM and other signals; signals in games.

## Follow-ups

- **A read interrupted over Ruby 1.8.7.** Ruby 1.8.7 raises `Interrupt` from its signal
  handler, with `longjmp`, also while it reads the standard input with `getc` and holds the
  lock of the C library's `stdin`; the interpreter thread then ends holding it, and `exit` on
  the main thread, which flushes every stream, would wait for it for ever. Resolved at the
  merge by the owner's session: the process of a script ends with `_exit` after the
  interpreter is finalised, flushing the standard output and error streams first
  (`vkrgss_guest::exit::end_script`); the scenario "Rescued" interrupts a read over both
  interpreters.
