## ADDED Requirements

### Requirement: A busy main thread lets a new thread run
Over Ruby 1.9.2 a thread that gives up the interpreter's lock at the end of its time slice
while another thread waits for it SHALL let that thread take it first, in the original
process and in a copy made by `fork` after `restart_thread_timer()`. A new thread SHALL so
run within a few time slices while the main thread loops in Ruby without blocking.

#### Scenario: Twenty threads in the original process
- **WHEN** a script twenty times sets `$ran = false`, starts `Thread.new { $ran = true }` and loops without blocking until `$ran` is set or thirty seconds have passed
- **THEN** `$ran` is `true` all twenty times

#### Scenario: A copy
- **WHEN** the scenario "A busy main thread after a copy" of the requirement "Script threads and copies of the process" is run 100 times in a row
- **THEN** every run passes, in the copy and in the original
