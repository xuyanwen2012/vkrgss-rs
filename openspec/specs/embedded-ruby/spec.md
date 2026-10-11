# embedded-ruby Specification

## Purpose
Defines the Ruby 1.8.7 interpreter embedded in vkxp: how it is built from vendored source, what
is true of it at start-up, and how host code enters and leaves it without corrupting either
side.

## Requirements

### Requirement: Hermetic build
Building the workspace SHALL compile the vendored interpreter sources and its `Zlib` extension
from a fixed list of files with fixed compiler flags, using only a C compiler. The build SHALL
NOT run the interpreter's `configure` or `make`, need an installed Ruby, or use a zlib installed
on the system. For a target without a provided configuration header the build SHALL fail with a
message that names the target.

#### Scenario: No Ruby on the machine
- **WHEN** the workspace is built on a machine with a C compiler and no Ruby installation
- **THEN** the build succeeds and produces the interpreter library

#### Scenario: Unsupported target
- **WHEN** the crate is built for a target that has no configuration header
- **THEN** the build stops with an error naming that target and the supported one

### Requirement: Vendored source matches upstream
Every vendored file SHALL be byte-identical to the file of the same path in the release
`ruby-1.8.7-p374.tar.gz` whose SHA-256 is
`876eeeaaeeab10cbf4767833547d66d86d6717ef48fd3d89e27db8926a65276c`, unless a local patch recorded
in the patch log changes it. A committed manifest SHALL hold the upstream hash of each vendored
file, and a test SHALL check the tree against the manifest and the patch log.

#### Scenario: Unmodified file
- **WHEN** the test hashes a vendored file that no patch touches
- **THEN** the hash equals the manifest's entry

#### Scenario: Patched file
- **WHEN** the test reaches a file changed by a recorded patch
- **THEN** it accepts the file only if its hash equals the patched hash recorded with that patch

#### Scenario: Unrecorded change
- **WHEN** a vendored file differs from the manifest and no patch is recorded for it
- **THEN** the test fails and names the file

### Requirement: Interpreter identity
The embedded interpreter SHALL identify itself as Ruby 1.8.7 patch level 374, SHALL read and
write Marshal format 4.8, and SHALL have the Ruby 1.8 behaviours that old scripts rely on.

#### Scenario: Version
- **WHEN** `RUBY_VERSION` and `RUBY_PATCHLEVEL` are evaluated
- **THEN** they are `"1.8.7"` and `374`

#### Scenario: Ruby 1.8 language points
- **WHEN** `x = 1; case x; when 1: :one; end`, `"abc"[0]` and `[1, [2, 3]].to_s` are evaluated
- **THEN** the results are `:one`, `97` and `"123"`

#### Scenario: Large integers
- **WHEN** `2**64` and `(2**64).class` are evaluated
- **THEN** the results are `18446744073709551616` and `Bignum`

### Requirement: Upstream test suites pass
Ruby's own core test suite, its `Zlib` test suite and its sample test SHALL pass against the
interpreter exactly as built for vkxp, and SHALL run as part of the workspace tests.

#### Scenario: Core suite
- **WHEN** the workspace tests run
- **THEN** the files of `test/ruby` report 625 tests with no failure and no error

#### Scenario: Zlib suite and sample
- **WHEN** the workspace tests run
- **THEN** `test/zlib` reports no failure and no error and `sample/test.rb` reports no `not ok`

### Requirement: Start-up state
After the interpreter is started through the crate's helper, `Zlib` SHALL be defined without
any `require`, a later `require 'zlib'` SHALL succeed without loading a file, the load path
SHALL be empty, and Ruby's standard output and error SHALL be unbuffered.

#### Scenario: Zlib without require
- **WHEN** `Zlib::Inflate.inflate(Zlib::Deflate.deflate("x" * 1000))` is the first thing evaluated
- **THEN** the result is the original string

#### Scenario: Requiring zlib later
- **WHEN** `require 'zlib'` is evaluated
- **THEN** it returns `false` and raises nothing

#### Scenario: Empty load path
- **WHEN** `$:` is evaluated after start-up
- **THEN** it is an empty array

### Requirement: Dedicated thread and bounded recursion
The interpreter SHALL run on one dedicated thread with a stack of at least 16 MiB, and all work
given to it from other threads SHALL be executed on that thread in the order given. Unbounded
recursion in a script SHALL raise `SystemStackError`, leave the interpreter usable and never
crash the process. A second interpreter in the same process SHALL be refused.

#### Scenario: Work from another thread
- **WHEN** two closures are submitted from a thread that is not the interpreter's
- **THEN** both run on the interpreter thread, the first before the second, and their results are returned

#### Scenario: Unbounded recursion
- **WHEN** a method that calls itself, a block that yields to itself and a string that evaluates itself are each run without a base case
- **THEN** each raises `SystemStackError` and a following evaluation of `1 + 1` returns 2

#### Scenario: Second interpreter
- **WHEN** a second interpreter is started while the first is alive
- **THEN** the request fails with an error and the first keeps working

### Requirement: Process signals are left alone
Starting the interpreter SHALL leave the process's dispositions for the interrupt, terminate,
hang-up, quit, user-defined, alarm, pipe, bus-error and segmentation-fault signals exactly as
they were before, including their flags and masks.

#### Scenario: Host handler survives
- **WHEN** the host installs a handler for the terminate signal with the restart flag and then starts the interpreter
- **THEN** the terminate signal still has that handler with that flag

#### Scenario: Defaults survive
- **WHEN** the interpreter is started in a process with default dispositions
- **THEN** every listed signal still has its default disposition afterwards

### Requirement: Protected evaluation
Host code SHALL be able to evaluate Ruby source under a given file name and first line number
and receive either its value or the raised exception's class name, message and backtrace. A
raised exception or a syntax error SHALL NOT unwind through host frames and SHALL leave the
interpreter usable.

#### Scenario: Value
- **WHEN** `6 * 7` is evaluated
- **THEN** the host receives the integer 42

#### Scenario: Exception
- **WHEN** a two-line source whose second line is `raise ArgumentError, 'boom'` is evaluated as file `Section001` from line 1
- **THEN** the host receives class `ArgumentError`, message `boom` and a backtrace whose first entry is `Section001:2`

#### Scenario: Syntax error
- **WHEN** `def (` is evaluated
- **THEN** the host receives a `SyntaxError` and the next evaluation works

### Requirement: Host functions callable from Ruby
Host code SHALL be able to define Ruby classes, modules, methods and global functions that are
implemented by host functions. A Ruby exception raised from inside a host function SHALL be
rescuable by Ruby code, and a panic inside a host function SHALL surface as a Ruby exception
instead of unwinding into the interpreter.

#### Scenario: Calling a host method
- **WHEN** a host function is registered as method `twice` of a new class and Ruby calls it with 21
- **THEN** Ruby receives 42

#### Scenario: Raising from a host function
- **WHEN** a host function raises `ArgumentError` and Ruby calls it inside `begin ... rescue ArgumentError`
- **THEN** the `rescue` branch runs

#### Scenario: Panic in a host function
- **WHEN** a host function panics while Ruby is calling it
- **THEN** Ruby sees an exception carrying the panic message and the process keeps running

### Requirement: Rooted values survive collection
A Ruby value that host code registers as a root SHALL stay alive across garbage collections for
as long as it is registered, even when no Ruby object and no stack slot refers to it.

#### Scenario: Rooted string
- **WHEN** a string is created, rooted, referenced only from host heap memory, and many collections are forced
- **THEN** the string still has its contents

### Requirement: Zlib is correct on 64-bit
`Zlib.crc_table` SHALL return the 256 values of the standard CRC-32 table.

#### Scenario: Table values
- **WHEN** `Zlib.crc_table` is evaluated
- **THEN** it has 256 entries starting `0x0`, `0x77073096`, `0xEE0E612C`, `0x990951BA`

#### Scenario: Table-driven checksum
- **WHEN** the CRC-32 of `"123456789"` is computed in Ruby from that table
- **THEN** it equals `Zlib.crc32("123456789")`, which is `0xCBF43926`

### Requirement: Interpreter symbols stay private
No symbol defined by the interpreter or its extension SHALL appear among the dynamic symbols
exported by an executable that links it.

#### Scenario: Exported symbols of a linked executable
- **WHEN** the dynamic symbol table of a test executable that links the interpreter is listed
- **THEN** it contains no defined symbol starting with `rb_`, `ruby_`, `st_` or `Init_`, and neither `dtoa` nor `re_set_syntax`

### Requirement: Random numbers as in Ruby 1.8.1
`srand(seed)` with an integer seed SHALL initialise the Mersenne Twister with that seed's low
32 bits by the generator's single-integer initialisation, and SHALL return the previous seed.
The generator's real number SHALL be one 32-bit output divided by 2^32. `rand(n)` for an
integer `n` other than zero SHALL return that real number multiplied by the absolute value of
`n` and truncated, and `rand` and `rand(0)` SHALL return that real number.

#### Scenario: Values measured on the original
- **WHEN** `srand(1234); (1..12).map { rand(1000) }` is evaluated
- **THEN** the result is `[191, 497, 622, 817, 437, 612, 785, 771, 779, 860, 272, 150]`

#### Scenario: Float
- **WHEN** `srand(1234); 12.times { rand(1000) }; rand` is evaluated
- **THEN** the result is `0.276464259717613` to fifteen digits

#### Scenario: Reference values
- **WHEN** `srand(5489); [rand(1000), rand(1000), rand(1000)]` is evaluated
- **THEN** the result equals the values computed in the test from a reference MT19937 initialised with 5489 by `init_genrand`, each 32-bit output divided by 2^32, times 1000, truncated

#### Scenario: Previous seed
- **WHEN** `srand(3); srand(4)` is evaluated
- **THEN** the second call returns `3`

### Requirement: Random number patch is recorded
The deviation of the random number functions from the vendored version SHALL be recorded with
the other patches.

#### Scenario: Recorded
- **WHEN** the manifest test of the vendored tree runs
- **THEN** `random.c` matches the hash recorded for its patched state

### Requirement: String hash as in Ruby 1.8.1
`String#hash` SHALL be computed as the original does: starting from 0, for each byte taken as
a signed 8-bit value, `key = key * 65599 + byte`; then `key = key + (key >> 5)` with an
arithmetic shift; all in wrapping signed 32-bit arithmetic; the result reduced to a 31-bit
signed Fixnum as on a 32-bit build. No random seed SHALL enter it, so that hashes with string
keys iterate in the original's order.

#### Scenario: String hashes measured on the original
- **WHEN** the hashes of `""`, `"a"`, `"ab"`, `"abc"`, `"foo"`, `"Graphics/Characters/001-Fighter01"`, `"\xe6\xb5\x8b\xe8\xaf\x95"`, `"\xff"` and `"\x80\x7f"` are evaluated
- **THEN** they are `0`, `100`, `6562051`, `833038373`, `876516207`, `-1017680531`, `97144166`, `-2` and `-8658938`

#### Scenario: Iteration order measured on the original
- **WHEN** a hash is filled with the keys `hp sp str dex agi int atk pdef mdef eva name id x y z` in that order and its keys are listed
- **THEN** the list is `name dex x mdef atk y pdef agi z int id eva str sp hp`

#### Scenario: Stable between runs
- **WHEN** `"abc".hash` is evaluated in two separate processes
- **THEN** both results are equal

### Requirement: Float hash as in Ruby 1.8.1
`Float#hash` SHALL be the sum of the eight bytes of the double, each taken as a signed 8-bit
value and multiplied by 971, made non-negative.

#### Scenario: Float hashes measured on the original
- **WHEN** `1.5.hash` and `0.0.hash` are evaluated
- **THEN** they are `53405` and `0`

### Requirement: Class redefinition with another superclass as in Ruby 1.8.1
A `class` statement that names an existing class together with an explicit superclass that is
not that class's superclass SHALL NOT raise: it SHALL create a new class with the given
superclass and bind the constant to it, leaving the old class object and its instances as they
were. A `class` statement without a superclass, or with the class's own superclass, SHALL
reopen the existing class. Naming a module in a `class` statement SHALL still raise
`TypeError`.

#### Scenario: Measured on the original
- **WHEN** `class KA; def x; end; end; class KB; end; old = KA; class KA < KB; def y; end; end` is evaluated
- **THEN** no error is raised, `KA` is not the same object as `old`, `KA.superclass` is `KB`, a new `KA` instance responds to `y` and not to `x`, and `old.name` is still `"KA"`

#### Scenario: Explicit Object
- **WHEN** `class KC < KB; end; class KC < Object; end` is evaluated
- **THEN** no error is raised and `KC.superclass` is `Object`

#### Scenario: Reopening
- **WHEN** `class KA; def z; end; end` is evaluated for an existing `KA`
- **THEN** `KA` is the same class object as before with the new method

#### Scenario: Module
- **WHEN** `module KM; end; class KM; end` is evaluated
- **THEN** a `TypeError` is raised

### Requirement: Symbol numbers as in the original runtime
A name that the original runtime has interned before the first game script SHALL have the same
symbol number in vkxp, taken from the committed table. Once the host has marked the start of
the game's scripts, a name seen for the first time SHALL get the next number of the sequence
the original continues with. A name interned before the mark that is not in the table SHALL
get a number outside that sequence, and its first use after the mark SHALL consume one number
of the sequence.

#### Scenario: Table names
- **WHEN** `:name.to_i`, `:id.to_i` and `:s.to_i` are evaluated
- **THEN** they equal the numbers the table lists for those names

#### Scenario: First new symbols, measured on the original
- **WHEN** the first script after the mark is `zz_first = 1; zz_second = 2` and it evaluates `[:zz_first.to_i, :zz_second.to_i]`
- **THEN** the result is `[16193, 16201]`

#### Scenario: Instance variable order
- **WHEN** an object is given the instance variables `@torch_keep`, `@message_frame`, `@magic_number` and `@map_interpreter` in an order of the test's choosing and `instance_variables` is evaluated twice in separate processes
- **THEN** both results are equal, and equal to the order computed in the test from the symbol numbers and the interpreter's table layout

#### Scenario: Name outside the table before the mark
- **WHEN** the host interns a name that is not in the table before the mark, and the first game script then creates a new local variable
- **THEN** that variable's symbol number is the first of the sequence, as if the earlier name did not exist

### Requirement: Regular expressions as in the original runtime
Regular expressions SHALL be compiled and matched by Oniguruma with Ruby syntax, in the
encoding that `$KCODE` and the expression's own encoding flag select, so that patterns the
original runtime accepts are accepted and match as they do there. The engine's source SHALL be
vendored with a manifest like the interpreter's, and the interpreter's former engine SHALL NOT
be built.

#### Scenario: Results measured on the original
- **WHEN** each committed pattern and subject measured on the original runtime is evaluated with `$KCODE` set to `UTF8`
- **THEN** the match position, the captures, the result of `gsub` and the number of `scan` results equal the measured ones, and a pattern that is an error on the original is a `SyntaxError` with the same message

#### Scenario: The pattern that stopped a game
- **WHEN** `/\[Vv][([0-9]+)]/` is evaluated
- **THEN** no error is raised

#### Scenario: Multi-byte characters
- **WHEN** `"测试".scan(/./).length` and `"测试".split(//).length` are evaluated with `$KCODE` set to `UTF8`
- **THEN** both are 2

#### Scenario: Interpreter suites
- **WHEN** the upstream test suites of the interpreter run
- **THEN** they pass, except cases that pin behaviour of the former engine, which are adjusted and listed with the patch

#### Scenario: Vendored source is pinned
- **WHEN** the manifest test of the vendored engine runs
- **THEN** every file matches its recorded hash

### Requirement: One protected call
The crate that embeds the interpreter SHALL offer the function that runs a C-level body under
the interpreter's protection and reports a raised exception as a value, and the crate that
defines the RGSS classes SHALL use it rather than its own.

#### Scenario: Body that returns
- **WHEN** a body run through the function returns a value
- **THEN** the caller receives that value

#### Scenario: Body that raises
- **WHEN** a body run through the function raises `ArgumentError`
- **THEN** the caller receives the exception as a value, no host frame was unwound, and `$!` is nil afterwards

#### Scenario: Binding behaviour unchanged
- **WHEN** the tests of the crate that defines the RGSS classes run
- **THEN** they pass as before

### Requirement: Upstream suites under sanitizers
The repository SHALL provide an opt-in run that builds the interpreter and the regular
expression engine with AddressSanitizer and UndefinedBehaviorSanitizer and runs the upstream
test suites under them. The run SHALL fail on a sanitizer report that is not in a committed
allow-list whose entries each state a reason. The run SHALL NOT be part of the merge gate.

#### Scenario: Sanitized run
- **WHEN** the opt-in run is started on the development machine
- **THEN** the upstream suites pass and the sanitizers report nothing outside the allow-list

#### Scenario: Not in the gate
- **WHEN** the merge gate runs
- **THEN** the sanitized build is not made

### Requirement: Upstream suites do not depend on the caller's signal dispositions
The harness that runs the upstream test suites SHALL start them with the default disposition
of the signals those suites send to themselves.

#### Scenario: SIGINT ignored by the caller
- **WHEN** the upstream suites are run from a process that ignores SIGINT
- **THEN** they pass as they do from a terminal

#### Scenario: Gate without a terminal
- **WHEN** `cargo xtask gate` is started with SIGINT ignored
- **THEN** its test step passes

### Requirement: Dumped strings survive a collection
`Marshal.dump` SHALL write the bytes that a `_dump` method returned, and the names of
classes and modules, also when a garbage collection runs while they are written, whichever
compiler built the interpreter.

#### Scenario: Table under stress
- **WHEN** `Marshal.dump(Table.new(20, 15, 3))` is evaluated with `GC.stress = true` by an interpreter built with gcc 13
- **THEN** the 20 bytes after the header unpack to 3, 20, 15, 3 and 900

#### Scenario: Patch is recorded
- **WHEN** `PATCHES.md` and the manifest test are checked
- **THEN** the change to `marshal.c` has an entry with its diff and patched hash, and the manifest test passes
