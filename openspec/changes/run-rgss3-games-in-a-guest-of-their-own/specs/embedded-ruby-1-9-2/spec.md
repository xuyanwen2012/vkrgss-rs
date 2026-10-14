## ADDED Requirements

### Requirement: Script threads and copies of the process
Both interpreter crates SHALL offer `threads_lost_in_a_copy()` and `restart_thread_timer()`,
to be called on the interpreter thread. Over Ruby 1.9.2 the first SHALL be true exactly
while a Ruby thread other than the main one is alive, and the second, called in a copy made
by `fork`, SHALL make script threads work there whether or not the copied process had made
one. Over Ruby 1.8.7 the first SHALL be false and the second what it is today.

#### Scenario: No thread, a live thread, an ended thread
- **WHEN** a host function reads `threads_lost_in_a_copy()` before a script made a thread, while `Thread.new { sleep }` is alive, and after that thread was killed and joined
- **THEN** the answers are `false`, `true` and `false`

#### Scenario: A thread after a copy
- **WHEN** a script evaluates `Thread.new { 1 + 1 }.value`, a host function then forks on the interpreter thread and the copy calls `restart_thread_timer()`, and the copy and the original each evaluate `t = Thread.new { sleep 0.05; :done }; t.join; t.value`
- **THEN** both give `:done` within five seconds and the copy exits with status 0

#### Scenario: A busy main thread after a copy
- **WHEN** the copy of that scenario instead starts `Thread.new { $ran = true }` and loops in Ruby without sleeping until `$ran` is set or thirty seconds have passed
- **THEN** `$ran` is `true`

#### Scenario: A copy without any thread
- **WHEN** the scenario "Fibers on the interpreter thread" of the requirement "A copy of the process between two frames" is run with `restart_thread_timer()` called in every copy
- **THEN** every copy and the original finish with the same digest

#### Scenario: Ruby 1.8.7
- **WHEN** a host function over Ruby 1.8.7 reads `threads_lost_in_a_copy()` while a script thread is alive
- **THEN** the answer is `false`, and the tests of the crate that hold `restart_thread_timer` pass unedited

#### Scenario: Documentation
- **WHEN** the crate documentation of `vkxp-ruby192-sys` is read
- **THEN** its table of differences from `vkxp-ruby-sys` no longer lists `restart_thread_timer` as absent and says what the two functions do over Ruby 1.9.2
