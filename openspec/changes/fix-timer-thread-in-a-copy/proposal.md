## Why

The test `fork_busy_main_after_copy` failed in about 4 of 15 runs on an idle machine (7 of 15
in a measurement of 2026-10-14): a script that creates a thread and then waits for it in a
loop that never blocks saw the thread not run within 30 seconds. The test runs the script in
a copy of the process made by `fork` and in the original, and both fail. The cause is the
global VM lock of Ruby 1.9.2, a plain mutex that the main thread takes again at the end of
every time slice before the waiting thread has woken.

## What Changes

- A recorded patch of `thread.c` (`gvl-handover`): a thread that waits for the lock is
  counted, and a thread that gives the lock up at the end of its time slice while another
  waits sleeps until that one has taken it.
- `restart_thread_timer()` of `vkxp-ruby192-sys` also forgets the waiters of the copied
  process, as `rb_thread_atfork` now does after the interpreter's own `fork`.
- A test, `busy_main_thread.rs`, holds the handover in the original process.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby-1-9-2`: a main thread that never blocks lets a new thread run, in the
  original process and in a copy.

## Impact

`third_party/ruby-1.9.2/thread.c`, `PATCHES.md` and a new patch file;
`crates/vkxp-ruby192-sys/src/signals.rs` and the crate documentation; a new test file of
`crates/vkxp-ruby192-sys`.
