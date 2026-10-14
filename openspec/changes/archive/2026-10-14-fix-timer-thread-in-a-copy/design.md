## Context

The change is named after the first suspect, the timer thread of a copy. Every timer
suspect was measured and excluded; the cause is elsewhere and not specific to a copy.

## Measured

Temporary prints on stderr (under an environment variable, in a clone only) in
`rb_thread_create_timer_thread`, `thread_timer`, `timer_thread_function`,
`rb_thread_schedule_rec` and `thread_start_func_2`, ten runs of the test:

- In a failing run the copy had created its timer thread (`timer_thread_id` was 0 after
  `restart_thread_timer`, a new thread was made), the thread ticked every 10 ms and set the
  interrupt of `vm->running_thread`, which was the main thread of the copy
  (`vm->main_thread`, the same structure); the timer thread never left its loop early.
- The main thread then called `rb_thread_schedule_rec` at every tick: 3000 times in 30
  seconds, each an unlock of the GVL, `sched_yield` and a lock. The new thread, blocked in
  `pthread_mutex_lock` of the GVL in `thread_start_func_2`, did not get it once.
- Of six failing runs, three failed in the original process, not in the copy (the test
  checks the copy first, so a failure of both shows as the copy's). The signal buffer was
  empty and no `raised_flag` was set.
- Twenty new threads in a row in the original process, each awaited by such a loop: seven
  had not run after 30 seconds, the others ran after 0.105 to 28.0 seconds. With the patch
  each ran after 0.010 seconds, one time slice.

Excluded so: a timer thread not started or thought running, an interrupt set on a stale
thread structure, a stale disposition of SIGVTALRM (no thread is woken by it here), and the
missing `rb_thread_atfork` of vkxp's copy (the original fails as well). Ruby 1.9.2 has no
waiting count or condition variable of the GVL (they came with 1.9.3); its GVL is a plain
mutex, which is not fair: the woken waiter loses the race to the thread that has just
unlocked.

## Decisions

- Patch `thread.c` as 1.9.3 does in spirit, with the mutex kept as the GVL so that no
  structure of the interpreter changes: `gvl_acquire` counts a waiting thread and counts each
  acquisition; `gvl_yield`, when a thread waits, unlocks and sleeps on a condition variable
  until the count of acquisitions has moved, then takes the lock again as a waiter. With no
  waiter it unlocks, yields and locks as before. Every place that took the GVL after the
  start (the end of a blocking region, a new thread, `GVL_UNLOCK_END`) uses `gvl_acquire`.
- The count belongs to the threads of one process. `ruby_vkxp_gvl_atfork` forgets it and
  makes the lock and condition variable of the handover afresh; `rb_thread_atfork` calls it
  in the interpreter's own child and `restart_thread_timer()` in vkxp's copy. It is not tied
  to `rb_thread_reset_timer_thread`, which the interpreter also calls in the parent after a
  `fork`, where the waiters are real.
