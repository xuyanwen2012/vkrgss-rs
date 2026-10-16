# embedded-ruby-1-9-2 Specification

## Purpose
Defines the Ruby 1.9.2 interpreter that vkrgss builds for RPG Maker VX Ace (RGSS3): how it is
built from vendored source beside Ruby 1.8.7 without the two meeting in one program, what is
true of it at start-up as measured on the original runtime, and how host code enters and
leaves it.

## Requirements

### Requirement: Hermetic build
On `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, building
`vkrgss-ruby192-sys` SHALL compile the vendored Ruby 1.9.2, its encodings and `Zlib` from a
fixed list of files with fixed flags and a checked-in configuration header, using only a C
compiler: no `configure`, `make`, installed Ruby, generated source or system zlib. On any
other target the crate SHALL build empty and its build script SHALL warn, naming the target.

#### Scenario: No Ruby on the machine
- **WHEN** the workspace is built on an x86_64 Linux machine with a C compiler and no Ruby installation
- **THEN** the build succeeds and produces the interpreter library

#### Scenario: Another target
- **WHEN** the crate is built for a target other than those two, such as `riscv64gc-unknown-linux-gnu`
- **THEN** the build succeeds without compiling a C file, the crate has no public item, and the build script prints a warning naming that target

#### Scenario: Header of aarch64
- **WHEN** the configuration header of `aarch64-unknown-linux-gnu` is compared with that of `x86_64-unknown-linux-gnu`
- **THEN** they differ only in `RUBY_PLATFORM` and the leading comment

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

### Requirement: Interpreter identity
The embedded interpreter SHALL identify itself as the original runtime's interpreter does,
except for the platform, which is `x86_64-linux`.

#### Scenario: Version constants, measured on the original
- **WHEN** `RUBY_VERSION`, `RUBY_PATCHLEVEL`, `RUBY_RELEASE_DATE`, `RUBY_REVISION`, `RUBY_ENGINE` and `RUBY_COPYRIGHT` are evaluated
- **THEN** they are `"1.9.2"`, `0`, `"2010-08-18"`, `29036`, `"ruby"` and `"ruby - Copyright (C) 1993-2010 Yukihiro Matsumoto"`

#### Scenario: Description and platform
- **WHEN** `RUBY_DESCRIPTION` and `RUBY_PLATFORM` are evaluated
- **THEN** they are `"ruby 1.9.2p0 (2010-08-18 revision 29036) [x86_64-linux]"` and `"x86_64-linux"`

### Requirement: Start-up state
After the interpreter is started through the crate's helper, its load path, loaded
features, default encodings and built-in modules SHALL be those measured on the original
runtime: `Zlib` is defined without being a loaded feature, the release's prelude has not
run, and the `fiber` and `continuation` extensions are absent. Source given to the crate's
evaluation helpers SHALL be read as UTF-8. Ruby's standard output and error SHALL be
unbuffered.

#### Scenario: Load path and features, measured on the original
- **WHEN** `$:` and `$LOADED_FEATURES` are evaluated after start-up
- **THEN** they are `["."]` and `["enumerator.so"]`

#### Scenario: Default encodings, measured on the original
- **WHEN** `Encoding.default_external`, `Encoding.default_internal`, `__ENCODING__` and `"abc".encoding` are evaluated
- **THEN** they are UTF-8, `nil`, UTF-8 and UTF-8

#### Scenario: Zlib without being a feature, measured on the original
- **WHEN** `Zlib::Inflate.inflate(Zlib::Deflate.deflate("x" * 1000))`, `Zlib::VERSION` and then `require "zlib"` are evaluated
- **THEN** the first is the original string, the second is `"0.6.0"`, and the third raises `LoadError`

#### Scenario: No prelude, measured on the original
- **WHEN** `Mutex.method_defined?(:synchronize)` and `Thread.respond_to?(:exclusive)` are evaluated
- **THEN** both are `false`

#### Scenario: Fibers of the core only, measured on the original
- **WHEN** `Fiber.new { Fiber.yield 1; 2 }` is resumed twice, and `Fiber.method_defined?(:alive?)`, `Fiber.respond_to?(:current)`, `Fiber.method_defined?(:transfer)` and `respond_to?(:callcc, true)` are evaluated
- **THEN** the two results are 1 and 2 and the four answers are `false`

### Requirement: Encodings as the original runtime has them
At start-up `Encoding.list` SHALL hold ASCII-8BIT, UTF-8, UTF-16LE and US-ASCII and no
other encoding. Shift_JIS, Windows-31J and ISO-8859-1 SHALL be usable by name without
being listed at start-up, with the transcoders between them and UTF-8 and between UTF-8
and UTF-16LE. No other encoding or transcoder of the release SHALL be linked, a name of
such an encoding SHALL be unknown, and looking a name up SHALL NOT touch the file system.

#### Scenario: Listed at start-up, measured on the original
- **WHEN** `Encoding.list.map(&:name).sort` is the first thing evaluated
- **THEN** it is `["ASCII-8BIT", "US-ASCII", "UTF-16LE", "UTF-8"]`

#### Scenario: Transcoding that works, measured on the original
- **WHEN** `"abc".encode("UTF-16LE")`, `"あ".encode("Shift_JIS")`, `"あ".encode("Windows-31J")` and `"é".encode("ISO-8859-1")` are evaluated and their bytes read as hexadecimal
- **THEN** the bytes are `610062006300`, `82a0`, `82a0` and `e9`

#### Scenario: Converter object, measured on the original
- **WHEN** `Encoding::Converter.new("UTF-8", "UTF-16LE").convert("ab")` is evaluated and its bytes read as hexadecimal
- **THEN** the bytes are `61006200`

#### Scenario: Unknown encodings, measured on the original
- **WHEN** `"あ".encode("EUC-JP")`, `"あ".encode("GBK")` and `"あ".encode("Big5")` are each evaluated
- **THEN** each raises an exception and the interpreter stays usable

#### Scenario: No file is looked for
- **WHEN** `"あ".encode("EUC-JP")` is evaluated in a directory that holds a file `enc/euc_jp.so` and with `"."` in the load path
- **THEN** it raises as in any other directory and the file is not opened

### Requirement: Fixed hash seed
The seed of `String#hash` and of the other hashes the interpreter seeds per process SHALL
be a constant of the crate, applied before the interpreter is initialised, through a
recorded patch.

#### Scenario: Two processes
- **WHEN** `["abc".hash, 1.5.hash, [1, 2].hash, :a.hash]` is evaluated in two separate processes
- **THEN** both processes give the same four numbers

#### Scenario: Insertion order, measured on the original
- **WHEN** `{:b => 1, :a => 2, "c" => 3}.keys` is evaluated
- **THEN** it is `[:b, :a, "c"]`

### Requirement: No timer thread before a script creates a thread
While the main thread is the only Ruby thread, the interpreter SHALL NOT have created any
operating-system thread. The interpreter's timer thread SHALL be created when a script
creates its first Ruby thread, and Ruby threads SHALL work from then on. The change SHALL
be a recorded patch.

#### Scenario: One Ruby thread
- **WHEN** an interpreter is started and evaluates a script that creates no Ruby thread, and the threads of the process are counted before the start and afterwards
- **THEN** the count has grown by exactly one, the interpreter thread that the crate created

#### Scenario: First script thread
- **WHEN** the script then evaluates `Thread.new { 1 + 1 }.value` and `t = Thread.new { sleep 0.05; :done }; t.join; t.value`
- **THEN** the results are `2` and `:done`

### Requirement: Dedicated thread and bounded recursion
The interpreter SHALL run on one thread that the crate creates, with a stack of at least
16 MiB, and work given to it from other threads SHALL run there in the order given. Its
limit of the machine stack SHALL follow the size of that thread's stack, not the resource
limit of the process. Unbounded recursion SHALL raise `SystemStackError`, leave the
interpreter usable and never crash the process. A second interpreter in the same process
SHALL be refused.

#### Scenario: Work from another thread
- **WHEN** two closures are submitted from a thread that is not the interpreter's
- **THEN** both run on the interpreter thread, the first before the second, and their results are returned

#### Scenario: Unbounded recursion
- **WHEN** a method that calls itself, a block that yields to itself, a lambda that calls itself and a string that evaluates itself are each run without a base case, in the main program and inside a Fiber
- **THEN** each raises `SystemStackError` and a following evaluation of `1 + 1` returns 2

#### Scenario: Stack limit of the process is larger than the thread's stack
- **WHEN** the soft limit of the stack size of the process is raised above the size of the interpreter thread's stack before the interpreter is started, where the hard limit allows it, and the recursions of the previous scenario run
- **THEN** each raises `SystemStackError` and the process does not crash

#### Scenario: Second interpreter
- **WHEN** a second interpreter is started while the first is alive
- **THEN** the request fails with an error and the first keeps working

### Requirement: Process signals are left alone
Starting the interpreter SHALL leave the disposition of every signal of the process as it
was before, including flags and masks, and SHALL leave the signal mask of the interpreter
thread as it was when the thread started.

#### Scenario: Host handler survives
- **WHEN** the host installs a handler for the terminate signal with the restart flag and then starts the interpreter
- **THEN** the terminate signal still has that handler with that flag

#### Scenario: Defaults and mask survive
- **WHEN** the interpreter is started in a process with default dispositions
- **THEN** every signal from 1 to 31 that can have a disposition still has its default, and on the interpreter thread no signal is blocked that was not blocked before `ruby_init`

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

### Requirement: Zlib is correct on 64-bit
`Zlib.crc_table` SHALL return the 256 values of the standard CRC-32 table.

#### Scenario: Table values
- **WHEN** `Zlib.crc_table` is evaluated
- **THEN** it has 256 entries starting `0x0`, `0x77073096`, `0xEE0E612C`, `0x990951BA`

#### Scenario: Table-driven checksum
- **WHEN** the CRC-32 of `"123456789"` is computed in Ruby from that table
- **THEN** it equals `Zlib.crc32("123456789")`, which is `0xCBF43926`

### Requirement: Interpreter symbols stay private
No symbol defined by the interpreter, its regular expression engine, its encodings or its
extension SHALL appear among the dynamic symbols exported by an executable that links it.

#### Scenario: Exported symbols of a linked executable
- **WHEN** the dynamic symbol table of a test executable that links the interpreter with `-rdynamic` is listed
- **THEN** it contains no defined symbol starting with `rb_`, `ruby_`, `st_`, `Init_`, `onig` or `Onig`

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

### Requirement: A copy of the process between two frames
A copy of the process made by `fork` from a host function on the interpreter thread, while
the main thread is the only Ruby thread, SHALL run the script on to the same result as the
process it was copied from, without anything being done about the interpreter's threads in
the copy and without `rb_thread_atfork`.

#### Scenario: Fibers on the interpreter thread
- **WHEN** a script runs three Fibers that each yield twelve times from 20, 40 and 60 calls deep, creates garbage, collects, draws seeded random numbers, hashes strings, transcodes a string to Windows-31J and dumps its state with `Marshal`, calls a host function once per loop, and that function forks at each of the loops 1 to 12
- **THEN** every copy and the original process finish with the same digest of the script's state and every copy exits with status 0

#### Scenario: The same under collector stress
- **WHEN** a shorter script of the same shape runs with `GC.stress = true` and the host function forks once
- **THEN** the copy and the original finish with the same digest

### Requirement: Marshal as the original runtime writes it
`Marshal.dump` SHALL write format 4.8 with the bytes the original runtime writes for the
values below, and `Marshal.load` SHALL label a string that was dumped without an encoding
ASCII-8BIT.

#### Scenario: Object, measured on the original
- **WHEN** an instance of a class `AceV0Obj` whose `initialize` assigns `@a = 1`, `@b = "x"`, `@c = 1.5`, `@d = :sym`, `@e = [nil, true]` in that order is dumped
- **THEN** the dump is, in hexadecimal, `04086f3a0d41636556304f626a0a3a07406169063a07406249220678063a0645543a0740636608312e353a0740643a0873796d3a0740655b073054`

#### Scenario: Strings, measured on the original
- **WHEN** `"abc"`, `"abc".force_encoding("ASCII-8BIT")`, `"abc".force_encoding("US-ASCII")`, `"abc".force_encoding("Shift_JIS")`, `"abc".encode("UTF-16LE")` and `"あ"` are dumped
- **THEN** the dumps are `0408492208616263063a064554`, `04082208616263`, `0408492208616263063a064546`, `0408492208616263063a0d656e636f64696e67220e53686966745f4a4953`, `040849220b610062006300063a0d656e636f64696e67220d5554462d31364c45` and `0408492208e38182063a064554`

#### Scenario: Floats, measured on the original
- **WHEN** `1.5`, `0.1`, `1e23`, `1.0 / 3`, `-0.0`, `1e100`, `1.0 / 0`, `0.0 / 0` and `100.0` are dumped
- **THEN** the dumps are `04086608312e35`, `0408661b302e313030303030303030303030303030303100999a`, `0408661e392e39393939393939393939393939393932652b3232004af6`, `0408661b302e3333333333333333333333333333333331005555`, `040866072d30`, `0408660e31652b31303000c37d`, `04086608696e66`, `040866086e616e` and `04086608313030`

#### Scenario: Symbols and other values, measured on the original
- **WHEN** `:sym`, `:"あ"`, `[:a, :a]`, `{1 => 2}`, `2**40`, `1..2`, `/a/i`, `nil`, `300` and `-129` are dumped
- **THEN** the dumps are `04083a0873796d`, `0408493a08e38182063a064554`, `04085b073a06613b00`, `04087b0669066907`, `04086c2b08000000000001`, `04086f3a0a52616e6765083a096578636c463a0a626567696e69063a08656e646907`, `0408492f066101063a064546`, `040830`, `040869022c01` and `040869ff7f`

#### Scenario: Load of a string without encoding, measured on the original
- **WHEN** `Marshal.load("\x04\b\"\babc".force_encoding("ASCII-8BIT")).encoding.name` is evaluated
- **THEN** it is `"ASCII-8BIT"`

### Requirement: Random numbers as the original runtime gives them
After `srand(n)`, `rand`, `Random`, `Array#shuffle` and `Array#sample` SHALL give the
values the original runtime gives, with the generator of the release unchanged.

#### Scenario: Integers and a float after srand(1), measured on the original
- **WHEN** `srand(1)` is followed by `rand(1000000)` five times and then `rand`
- **THEN** the results are 128037, 491755, 470924, 791624, 491263 and `0.12812444792935673`

#### Scenario: Further draws of that sequence, measured on the original
- **WHEN** the same sequence continues with `rand(10)`, `rand(2**40)`, `rand(1.5).to_i` and `rand(-5).to_i`
- **THEN** the results are 5, 340316410816, 0 and 0

#### Scenario: Floats after srand(1), measured on the original
- **WHEN** `srand(1)` is followed by `rand` three times
- **THEN** the results are `0.417022004702574`, `0.7203244934421581` and `0.00011437481734488664`

#### Scenario: Other seeds, measured on the original
- **WHEN** `srand(5)` is followed by `rand(1000)` six times, and `srand(20261009)` by `rand(1000000)` five times and `"%.17g" % rand`
- **THEN** the results are 867, 206, 701, 998, 118, 400, then 452058, 88078, 612515, 880807, 883900 and `"0.11596889136695721"`

#### Scenario: Random objects, measured on the original
- **WHEN** `r = Random.new(1)` gives `r.rand(1000000)` five times, `r.rand` and `r.bytes(4)` read as hexadecimal
- **THEN** the results are 128037, 491755, 470924, 791624, 491263, `0.12812444792935673` and `851ec1ff`

#### Scenario: Shuffle and sample, measured on the original
- **WHEN** `srand(1)` is followed by `(1..10).to_a.shuffle` and `(1..10).to_a.sample`
- **THEN** the results are `[9, 4, 2, 10, 6, 8, 3, 1, 7, 5]` and 5

### Requirement: Float text and rounding as the original runtime gives them
`Float#to_s` and rounding SHALL give the results of the original runtime for the values
below.

#### Scenario: Float text, measured on the original
- **WHEN** `0.1 + 0.2`, `1e23`, `1.0 / 3`, `1e22`, `1e16`, `1e15`, `100.0`, `1e-5`, `1e-4`, `1e300`, `-0.0`, `2.0**70` and `5e-324` are converted with `to_s`
- **THEN** the texts are `0.30000000000000004`, `1.0e+23`, `0.3333333333333333`, `1.0e+22`, `1.0e+16`, `1000000000000000.0`, `100.0`, `1.0e-05`, `0.0001`, `1.0e+300`, `-0.0`, `1180591620717411300000.0` and `5.0e-324`

#### Scenario: Rounding and division, measured on the original
- **WHEN** `2.5.round`, `3.5.round`, `(-2.5).round`, `2.675.round(2)`, `10.divmod(3)`, `-7 / 2`, `-7 % 3` and `7.fdiv(2)` are evaluated
- **THEN** the results are 3, 4, -3, 2.68, `[3, 1]`, -4, 2 and 3.5

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

### Requirement: Hardening flags are asked for by the build script
The build script of the interpreter SHALL itself give the compiler `-D_FORTIFY_SOURCE=3`,
`-fstack-protector-strong` and `-fstack-clash-protection`, and on x86_64
`-fcf-protection=full`, whatever the compiler's defaults are. Builds for the sanitizers or
fuzzing MAY leave out `_FORTIFY_SOURCE`.

#### Scenario: Flags of the build
- **WHEN** the flags that `base_build()` of the build script gives the compiler are read
- **THEN** they contain those flags, and the x86_64 one only for that architecture

#### Scenario: The suites pass with them
- **WHEN** the tests of the interpreter crate run with the default compiler of this machine
- **THEN** they pass

### Requirement: Sanitized run of the upstream suite
`vkrgss-ruby192-sys` SHALL have a test, ignored by default, that builds Ruby 1.9.2 with clang,
AddressSanitizer and UndefinedBehaviorSanitizer and runs the upstream suite with it. Every
report SHALL fail the run unless an allow-list in the crate names it with its reason.

#### Scenario: A clean run
- **WHEN** `cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored` is run on a machine with clang and its sanitizer runtimes
- **THEN** it passes, and every report it saw is on the allow-list

### Requirement: Fuzz targets of Ruby 1.9.2
`cargo xtask fuzz` SHALL also build and run three targets of Ruby 1.9.2, `marshal192`,
`parse192` and `regex192`, that feed arbitrary bytes to `Marshal.load`, to the parser and
to the regular expression engine. A Ruby exception SHALL NOT count as a failure. An input
that made a target fail and was fixed SHALL be kept as a test of the normal test step.

#### Scenario: A short run
- **WHEN** `cargo xtask fuzz marshal192 --seconds 30` is run on a machine with clang and its sanitizer runtimes
- **THEN** the target runs for about 30 seconds and ends with status 0 when nothing failed

#### Scenario: Unknown target
- **WHEN** `cargo xtask fuzz nothing` is run
- **THEN** it ends with an error that names all six targets

### Requirement: Later security fixes of Ruby 1.9.2 are reviewed
`docs/vendored-security.md` SHALL list the security fixes published for Ruby after
1.9.2-p0 that concern code the tree of Ruby 1.9.2 builds, and for each whether a game's
data reaches it and whether the fix was backported as a recorded patch, does not apply, or
was left, with the reason.

#### Scenario: A row for each
- **WHEN** the section on Ruby 1.9.2 of `docs/vendored-security.md` is read
- **THEN** every row has an identifier, the code concerned and one of the three outcomes with its reason or patch number

#### Scenario: Backports are recorded
- **WHEN** a row says backported
- **THEN** `PATCHES.md` of the tree of Ruby 1.9.2 has the patch and the vendored-source checks pass

### Requirement: Backward searches check the least length of a match
The regular expression engine SHALL report no match from a backward search whose pattern's
least length of a match exceeds the length of the subject, before it forms any position
from that length.

#### Scenario: Pattern longer than the subject
- **WHEN** `"ab".rindex(/abc/)` and backward searches with a literal prefix, a character class and an anchor, each with a least length above the subject's length, are evaluated
- **THEN** each returns `nil`

#### Scenario: Patch is recorded
- **WHEN** the vendored source of the engine is checked against its manifest
- **THEN** the change to `backward_search_range` is a patch recorded in `PATCHES.md` that names upstream's commit 33e5b47c

### Requirement: Mathematical functions are correctly rounded
For a regular argument, the functions of `Math` and the power of floats SHALL return the
correctly rounded result, computed by the vendored CORE-MATH, on every platform. Special
arguments and errors SHALL behave as before.

#### Scenario: Test vectors
- **WHEN** each line of the committed test vectors for a function the interpreter has is evaluated through `Math` or `**`
- **THEN** the result has exactly the bits of the line's result

#### Scenario: Special arguments
- **WHEN** `Math.log(-1)`, `Math.sqrt(-1)`, `Math.acos(2)`, `Math.atanh(1)`, `Math.log(0)`, `Math.exp(1000)` and `0.0 ** -1` are evaluated
- **THEN** each raises the same error or returns the same value as before this requirement

### Requirement: No contraction of floating-point operations
The interpreter and the vendored CORE-MATH SHALL be compiled with `-ffp-contract=off`.

#### Scenario: Flags of the build
- **WHEN** the flags of the build script are read
- **THEN** they contain `-ffp-contract=off` for the interpreter and for CORE-MATH

### Requirement: Values held only by an inner pointer survive an allocation
Where the code of Ruby 1.9.2 reads an object through a pointer into it after a call that can
allocate, the object SHALL be kept alive until that read, on every supported architecture.

#### Scenario: Arithmetic of two-digit Bignums under stress
- **WHEN** `(2**63 - 1) + 1` and `(2**63 - 1) - (-1)` are evaluated with `GC.stress = true`
- **THEN** both equal `2**63`

#### Scenario: Time at a file under stress
- **WHEN** `File.utime` is called with `Time.local(2000, 1, 1)` for a file, with `GC.stress = true`
- **THEN** it succeeds

#### Scenario: Marshal of a Time with a fraction under stress
- **WHEN** `Marshal.dump` of `Time.at(0, Rational(1, 3))` and of `Time.at(0, 0.123)` runs with `GC.stress = true`
- **THEN** each returns a string that `Marshal.load` turns back into an equal time

#### Scenario: Patches are recorded
- **WHEN** the vendored source is checked against its manifest
- **THEN** every guard added is part of a patch recorded in `PATCHES.md`
