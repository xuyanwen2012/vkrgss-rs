## RENAMED Requirements

- FROM: `### Requirement: The Rust API of vkxp-ruby-sys`
- TO: `### Requirement: The Rust API of vkrgss-ruby-sys`

## MODIFIED Requirements

### Requirement: Hermetic build
On the target `x86_64-unknown-linux-gnu`, building `vkrgss-ruby192-sys` SHALL compile the
vendored Ruby 1.9.2, its linked encodings and its `Zlib` extension from a fixed list of
files with `-std=gnu89`, `-fwrapv` and `-fno-strict-aliasing`, using only a C compiler and
a checked-in configuration header: no `configure`, no `make`, no installed Ruby, no
generated source, no zlib of the system. On any other target the crate SHALL build empty
and its build script SHALL warn, naming the target.

#### Scenario: No Ruby on the machine
- **WHEN** the workspace is built on an x86_64 Linux machine with a C compiler and no Ruby installation
- **THEN** the build succeeds and produces the interpreter library

#### Scenario: Another target
- **WHEN** the crate is built for `aarch64-unknown-linux-gnu`
- **THEN** the build succeeds without compiling a C file, the crate has no public item, and the build script prints a warning naming that target

### Requirement: Vendored source matches upstream
Every vendored file of the release SHALL be byte-identical to the file of the same path in
`ruby-1.9.2-p0.tar.gz`, whose SHA-256 is
`8c0c4e261a921b5c406bf9e76ac23bf3c915651534e9d1b9e8c5d0bee4a7285c`, unless a patch recorded
in the patch log changes it. A committed manifest SHALL hold the upstream hash of each
file, each log entry SHALL state file, diff, patched hash and reason, and a test SHALL
check the tree against both.

#### Scenario: Unmodified file
- **WHEN** the test hashes a vendored file that no patch touches
- **THEN** the hash equals the manifest's entry

#### Scenario: Patched file
- **WHEN** the test reaches a file changed by a recorded patch
- **THEN** it accepts the file only if its hash equals the patched hash recorded with that patch

#### Scenario: Unrecorded change
- **WHEN** a vendored file differs from the manifest and no patch is recorded for it, or a file is present that neither the manifest nor the list of files added by vkrgss names
- **THEN** the test fails and names the file

### Requirement: The Rust API of vkrgss-ruby-sys
The crate SHALL offer the public modules and items of `vkrgss-ruby-sys` under the same names
with the same meaning. An item without a counterpart in Ruby 1.9.2 SHALL be absent and
listed in the crate documentation with what takes its place; `mark_game_scripts` SHALL
exist and do nothing. What `ffi` mirrors SHALL be that of the vendored headers, and a test
SHALL compare the two. `str_new` SHALL make an ASCII-8BIT string and the constructors for
text a UTF-8 string.

#### Scenario: Public items
- **WHEN** a test crate names `ffi`, `Interpreter`, `Options`, `StartError`, `Ruby`, `protect`, `Raised`, `RubyError`, `HostResult`, `Raise`, `Arity`, `Root`, `MIN_STACK_SIZE`, `STACK_HEADROOM`, `GC_STRESS_VARIABLE` and `SIGNALS_TAKEN_BY_RUBY` of `vkrgss_ruby192_sys`, and the methods `eval`, `eval_protected`, `funcall`, `describe`, `set_current_error`, `str_new`, `str_new_text`, `str_new_utf8`, `str_bytes`, `inspect`, `root`, `check_arity`, `top_self`, `mark_game_scripts` and the `define_*` methods of `Ruby`, `RUBY_API` and `Options::for_rgss`
- **THEN** it compiles, and the crate documentation lists `ruby_errinfo`, `ruby_top_self`, `ruby_set_stack_size`, the four `ruby_vkxp_` hooks of 1.8.7, `SymbolTable`, `rstring` and `rarray` as absent, each with what takes its place

#### Scenario: Value
- **WHEN** `6 * 7` is evaluated through `eval_protected`
- **THEN** the host receives the integer 42

#### Scenario: Exception
- **WHEN** a two-line source whose second line is `raise ArgumentError, 'boom'` is evaluated as file `Section001` from line 1
- **THEN** the host receives class `ArgumentError`, message `boom` and a backtrace whose first entry starts with `Section001:2`

#### Scenario: Syntax error
- **WHEN** `def (` is evaluated
- **THEN** the host receives a `SyntaxError` and the next evaluation works

#### Scenario: Host method, raise and panic
- **WHEN** host functions are registered as methods of a new class, one that doubles its argument, one that raises `ArgumentError` and one that panics, and Ruby calls the first with 21, the second inside `begin ... rescue ArgumentError` and the third inside `begin ... rescue Exception`
- **THEN** Ruby receives 42, the first `rescue` branch runs, and the second `rescue` branch sees an exception carrying the panic message

#### Scenario: Fixed arity
- **WHEN** a host function is registered with a fixed arity of 2 and Ruby calls it with one argument and asks `method(:name).arity`
- **THEN** the call raises `ArgumentError` before the body runs and the arity is 2

#### Scenario: Rooted string
- **WHEN** a string is created, rooted, referenced only from host heap memory, and many collections are forced
- **THEN** the string still has its contents

#### Scenario: Bytes and text
- **WHEN** the host makes a string of the bytes `e3 81 82` with `str_new` and one with the text constructor and Ruby evaluates `encoding.name` and `size` of each
- **THEN** the first gives `"ASCII-8BIT"` and 3 and the second `"UTF-8"` and 1, and `str_bytes` returns the three bytes for both

#### Scenario: Embedded and heap values
- **WHEN** the host reads a string of 3 bytes and one of 100 bytes, and an array of 2 elements and one of 100, through the string and array accessors of `ffi`
- **THEN** it reads the lengths and contents that Ruby sees for each

#### Scenario: Headers and declarations agree
- **WHEN** the test compiles a C program against the vendored headers with the flags of the build
- **THEN** the sizes, offsets, type tags, flag bits and special constants it prints equal those of `ffi`, and a C file that declares every function of `ffi` with the types `ffi` gives it compiles against the headers without a conflicting declaration

### Requirement: The two interpreters are never linked together
No package of the workspace SHALL depend on both `vkrgss-ruby-sys` and `vkrgss-ruby192-sys`,
directly or transitively, where normal, build and development dependencies all count.
`vkrgss-ruby192-sys` SHALL be a guest crate under the dependency rules of the workspace. The
check SHALL be part of the dependency rules that the workspace tests run.

#### Scenario: The real workspace
- **WHEN** the dependency rules are checked over the workspace
- **THEN** no package reaches both crates and the check passes

#### Scenario: A package that reaches both
- **WHEN** the rule is given a graph in which one package reaches `vkrgss-ruby-sys` through a normal dependency and `vkrgss-ruby192-sys` through a development dependency
- **THEN** it reports a violation that names the package and both paths

### Requirement: Upstream test suites pass
The release's `bootstraptest` and a selection of its `test/ruby` SHALL run in the workspace
tests against an executable linked from the interpreter as built for vkrgss, with no further
encoding. A committed list SHALL name each file or test left out or allowed to fail, with
its reason: a cause in the environment or in thread scheduling that the study of
2026-10-13 explains, or something vkrgss does not link. The harness SHALL fail on a file that
reports no test.

#### Scenario: Bootstrap tests
- **WHEN** the workspace tests run
- **THEN** every file of `bootstraptest` that the list does not leave out reports no failure

#### Scenario: Core suite
- **WHEN** the workspace tests run
- **THEN** every selected file of `test/ruby` reports at least one test and no failure or error outside the list

#### Scenario: Reasons
- **WHEN** the list is parsed
- **THEN** every entry has a reason, and an entry without one fails a test

#### Scenario: SIGINT ignored by the caller
- **WHEN** the suites are run from a process that ignores SIGINT
- **THEN** they pass as they do from a terminal

#### Scenario: Search path of the caller
- **WHEN** the suites are run from a process whose `PATH` holds a directory that everybody may write
- **THEN** they pass as they do otherwise

### Requirement: Collector stress
With `VKRGSS_GC_STRESS` set to a number of 1 or more, an interpreter started through the crate
SHALL collect at every allocation after start-up; unset, empty or 0 SHALL change nothing.
The selected files of `test/ruby` SHALL also run with `GC.stress = true`: the quick ones in
the workspace tests, the slow ones as a step of `cargo xtask stress`, which SHALL stay
outside the gate. A file left out SHALL be named in a committed list with the reason.

#### Scenario: Every allocation
- **WHEN** an interpreter is started with `VKRGSS_GC_STRESS=1` and a script allocates 100 objects
- **THEN** `GC.count` has grown by at least 100

#### Scenario: Off
- **WHEN** an interpreter is started without the variable and the same script runs
- **THEN** `GC.stress` is `false` and `GC.count` has grown by less than 100

#### Scenario: Stressed selection in the workspace tests
- **WHEN** the workspace tests run
- **THEN** every selected file whose stressed run took under 60 seconds in the study of 2026-10-13 reports at least one test and no failure or error outside the list of allowed failures

#### Scenario: Slow files
- **WHEN** `cargo xtask stress` runs its step for this interpreter
- **THEN** the slow files report no failure or error outside the list, and `test_integer_comb.rb`, `test_m17n_comb.rb` and `test_yield.rb` are named in `xtask/src/stress.skip` with the reason that they did not finish in 14 minutes under stress

#### Scenario: Host frames across Fiber switches
- **WHEN** a host function keeps eleven new strings in local variables only, forces a collection, yields to a block that suspends the running Fiber, and reads the strings after the Fiber is resumed, in four Fibers with six calls each under `GC.stress = true`
- **THEN** all 24 calls return the contents they created

### Requirement: Fortified build
The interpreter of Ruby 1.9.2 SHALL pass the suites of the release that the workspace tests
run, plain and under collector stress, when the vendored source is compiled with
`-O2 -D_FORTIFY_SOURCE=3`, and the build script SHALL NOT switch that check off for the
library. Each correction SHALL be a patch recorded in `PATCHES.md` that says whether it fixes
a write past a buffer or rewrites an idiom the check misreads.

#### Scenario: The three files
- **WHEN** `cargo test -p vkrgss-ruby192-sys --locked --test upstream` is run with `CFLAGS='-O2 -D_FORTIFY_SOURCE=3'` in a target directory of its own
- **THEN** it passes, and `test_dir.rb`, `test_require.rb` and `test_system.rb` are not among the files left out or allowed to fail for this reason

#### Scenario: Under collector stress
- **WHEN** `cargo test -p vkrgss-ruby192-sys --locked --test upstream_stress` is run with the same flags
- **THEN** it passes

#### Scenario: The plain build
- **WHEN** `cargo test -p vkrgss-ruby192-sys --locked` is run without those flags
- **THEN** it passes as before, and the test of the manifest accepts the corrected files with their patches

#### Scenario: The check stays on
- **WHEN** the flags that the build script gives the compiler are read
- **THEN** none undefines `_FORTIFY_SOURCE` or sets it to 0 for the library as a whole

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
- **WHEN** the crate documentation of `vkrgss-ruby192-sys` is read
- **THEN** its table of differences from `vkrgss-ruby-sys` no longer lists `restart_thread_timer` as absent and says what the two functions do over Ruby 1.9.2

### Requirement: An interpreter without a script name
The interpreter crate SHALL start an interpreter whose options have an empty script name
without giving it a script name: `$0` SHALL then be `false` and the frame of a C function
at the bottom of a backtrace SHALL be named `ruby`. The options for a game of RGSS3 SHALL
have an empty script name; the default options and those for RGSS1 and RGSS2 SHALL keep
`vkrgss`.

#### Scenario: Options for RGSS3
- **WHEN** an interpreter started with `Options::for_rgss(3)` evaluates `[$0, $PROGRAM_NAME]` and the last entry of `caller(0)` inside a string given to `eval` by the crate's evaluation helper
- **THEN** the results are `[false, false]` and an entry that starts with ``ruby:in `eval'``

#### Scenario: Default options
- **WHEN** an interpreter started with the default options evaluates `$0`
- **THEN** the result is `"vkrgss"`, and the existing tests of the crate pass unedited
