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
  lock of the C library's `stdin`. `ruby` ends on the thread that holds the lock, which
  takes it again. In script mode the interpreter thread holds it and ends; the main thread
  then waits for the lock for ever in `exit`, which flushes every stream. So a script of
  `rgss1` or `rgss2` that rescues an `Interrupt` raised in `gets` runs to its end and the
  process does not end (SIGINT is blocked on the main thread; SIGTERM ends it). An
  `Interrupt` that is not rescued ends the process by SIGINT, which flushes nothing, and is
  not affected. A fix would end the process of a script without flushing the streams of the
  C library (`_exit` after the interpreter is finalised, which has flushed what Ruby wrote),
  or on the interpreter thread; neither is named above, so neither is built. The test of the
  scenario "Rescued" interrupts a read only over Ruby 1.9.2.
