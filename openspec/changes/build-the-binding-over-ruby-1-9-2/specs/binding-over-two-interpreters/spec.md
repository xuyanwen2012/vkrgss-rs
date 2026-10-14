## ADDED Requirements

### Requirement: One source of the classes and two packages
The RGSS classes SHALL have one source, `crates/vkxp-binding/src`. The package
`vkxp-binding-rgss3` SHALL compile that source over `vkxp-ruby192-sys` and SHALL hold no
source of the classes. The two packages SHALL declare the same dependencies apart from the
interpreter crate. The source SHALL NOT tell the interpreters apart by a `cfg` or a cargo
feature. On a target for which `vkxp-ruby192-sys` is empty the second package SHALL build
and be empty.

#### Scenario: Two libraries from one source
- **WHEN** `cargo build -p vkxp-binding -p vkxp-binding-rgss3 --locked` runs for `x86_64-unknown-linux-gnu`
- **THEN** both build, and the library of `vkxp-binding-rgss3` is compiled from `crates/vkxp-binding/src/lib.rs`

#### Scenario: No source of its own
- **WHEN** the directory `crates/vkxp-binding-rgss3` is listed
- **THEN** it has no directory `src`

#### Scenario: A dependency in one package only
- **WHEN** a dependency is declared by one of the two packages and not by the other
- **THEN** a test of the workspace fails and names the dependency

#### Scenario: One interpreter each
- **WHEN** the dependency rules are checked over the workspace with development dependencies counted
- **THEN** `vkxp-binding` reaches `vkxp-ruby-sys` and not `vkxp-ruby192-sys`, `vkxp-binding-rgss3` reaches `vkxp-ruby192-sys` and not `vkxp-ruby-sys`, and the check passes

#### Scenario: A package that reaches both binding packages
- **WHEN** the rule on the two interpreters is given a graph in which one package reaches `vkxp-binding` and `vkxp-binding-rgss3`
- **THEN** it reports a violation that names the package and both paths

#### Scenario: A target without the second interpreter
- **WHEN** `vkxp-binding-rgss3` is built and tested with the configuration that its build script sets for a target other than `x86_64-unknown-linux-gnu`
- **THEN** the build succeeds, its library has no item and each of its test binaries runs no test

#### Scenario: Documentation once
- **WHEN** the documentation step of the gate runs
- **THEN** it renders the documentation of `vkxp-binding` and none of `vkxp-binding-rgss3`

### Requirement: Common surface of the interpreter crates
`vkxp-ruby-sys` and `vkxp-ruby192-sys` SHALL offer every item that the source of the
binding and its tests name, under one name and with one signature. Both SHALL have a
constant that says which Ruby they are, the options for a runtime, the main object, the
accessors of strings and arrays, a constructor for bytes and one for text, and the mark of
the game's scripts. The additions to `vkxp-ruby-sys` SHALL change nothing
of what Ruby 1.8.7 does.

#### Scenario: The source compiles over both without a warning
- **WHEN** `cargo clippy -p vkxp-binding -p vkxp-binding-rgss3 --all-targets --locked -- -D warnings` runs
- **THEN** it passes

#### Scenario: Which interpreter
- **WHEN** a test of each interpreter crate reads `RUBY_API`
- **THEN** `vkxp-ruby-sys` gives the value of Ruby 1.8 and `vkxp-ruby192-sys` the value of Ruby 1.9

#### Scenario: Options for a runtime over Ruby 1.8.7
- **WHEN** `Options::for_rgss` of `vkxp-ruby-sys` is asked for 1, 2 and 3
- **THEN** the symbol table is that of RGSS1 for 1 and that of RGSS2 for 2 and 3, and every other field is the default

#### Scenario: Options for a runtime over Ruby 1.9.2
- **WHEN** `Options::for_rgss` of `vkxp-ruby192-sys` is asked for 1, 2 and 3
- **THEN** each answer has the fields of the default options

#### Scenario: The main object
- **WHEN** `inspect` is called on what `Ruby::top_self` returns, in each interpreter crate
- **THEN** the result is `main`

#### Scenario: Accessors over Ruby 1.8.7
- **WHEN** the host reads a string of 3 bytes and one of 100 bytes, and an array of 2 elements and one of 100, through `rstring_ptr`, `rstring_len`, `rarray_ptr` and `rarray_len` of `vkxp-ruby-sys`
- **THEN** it reads the lengths and contents that Ruby sees for each

#### Scenario: Text and bytes over Ruby 1.9.2
- **WHEN** the host makes a string of the bytes `e3 81 82` with `str_new` and one with `str_new_text` and Ruby evaluates `encoding.name` and `size` of each
- **THEN** the first gives `"ASCII-8BIT"` and 3 and the second `"UTF-8"` and 1

#### Scenario: Text that is not UTF-8 over Ruby 1.9.2
- **WHEN** the host makes a string of the one byte `ff` with `str_new_text`
- **THEN** the string is labelled UTF-8, `valid_encoding?` is false and `str_bytes` returns that byte

#### Scenario: Text and bytes over Ruby 1.8.7
- **WHEN** the host makes a string of the bytes `e3 81 82 ff` with `str_new` and one with `str_new_text` of `vkxp-ruby-sys`
- **THEN** the two are equal for Ruby and `Marshal.dump` of each gives the same bytes

#### Scenario: The mark over Ruby 1.9.2
- **WHEN** `mark_game_scripts` of `vkxp-ruby192-sys` is called twice between two evaluations
- **THEN** it returns and the second evaluation gives its value

#### Scenario: Ruby 1.8.7 as before
- **WHEN** the tests of `vkxp-ruby-sys` run
- **THEN** every test that passed before passes under its name with its expected values unedited

### Requirement: Strings the binding creates are text or bytes
Every Ruby string the binding creates SHALL be made through one of two functions: one for
bytes, which over Ruby 1.9.2 gives an ASCII-8BIT string, and one for text, which gives a
UTF-8 string. Over Ruby 1.8.7 the two SHALL give the same string. Text SHALL be only script
source handed to the interpreter and the path of the script archive. Everything else SHALL
be bytes: serialized forms, `to_s`, font names, data for `Marshal`, buffers, file names.

#### Scenario: Serialized values
- **WHEN** over Ruby 1.9.2 a script evaluates `_dump(0).encoding.name` of a `Color`, a `Tone`, a `Rect` and a `Table`, and the third byte of `Marshal.dump` of each
- **THEN** every name is `"ASCII-8BIT"` and every third byte is `u`, the type of a user-defined dump without instance variables

#### Scenario: Values as strings
- **WHEN** over Ruby 1.9.2 a script evaluates `Color.new(1, 2, 3, 255).to_s` and `Rect.new(1, 2, 3, 4).to_s` with the name of the encoding of each
- **THEN** the strings are `(1.000000, 2.000000, 3.000000, 255.000000)` and `(1, 2, 3, 4)` and both names are `"ASCII-8BIT"`

#### Scenario: Font name
- **WHEN** over Ruby 1.9.2 a script evaluates `Font.new.name.encoding.name`
- **THEN** the result is `"ASCII-8BIT"`

#### Scenario: Message of an error of RGSS
- **WHEN** over Ruby 1.9.2 a script disposes a bitmap, calls `clear` on it and rescues the error
- **THEN** the message is `disposed bitmap` and the name of its encoding is `"ASCII-8BIT"`

#### Scenario: Script source is text
- **WHEN** over Ruby 1.9.2 a game is run whose built-in scripts file and whose preload script each store the name of the encoding and the size of a literal of one character outside ASCII
- **THEN** both store `"UTF-8"` and 1

#### Scenario: Round trip
- **WHEN** over Ruby 1.9.2 a script evaluates `Marshal.load(Marshal.dump(value)) == value` for a `Color`, a `Tone`, a `Rect` and a `Table` with values of its own
- **THEN** every result is true

### Requirement: Preparation by interpreter
What the binding prepares before a game's scripts run SHALL be the same over both
interpreters, except that only over Ruby 1.8.7 it sets `$KCODE` and empties the load path.
The binding SHALL learn which interpreter it runs on from the constant of the interpreter
crate, in one place of its source. Over Ruby 1.9.2 the preparation SHALL define nothing of
the prelude of the Ruby release.

#### Scenario: Ruby 1.8.7
- **WHEN** a snippet evaluates `[$KCODE, $:]` after the preparation over Ruby 1.8.7
- **THEN** the result is `["UTF8", []]`

#### Scenario: Ruby 1.9.2
- **WHEN** a snippet evaluates `[$KCODE, $:]` after the preparation over Ruby 1.9.2
- **THEN** the result is `[nil, ["."]]`

#### Scenario: No prelude of the release
- **WHEN** a snippet evaluates `[Mutex.new.respond_to?(:synchronize), Thread.respond_to?(:exclusive)]` after the preparation over Ruby 1.9.2
- **THEN** the result is `[false, false]`

#### Scenario: What both prepare
- **WHEN** a snippet evaluates `[Reset.superclass, defined?(save_data), defined?(load_data)]` after the preparation over each interpreter
- **THEN** both give `Exception` and two values that are not `nil`

### Requirement: The tests of the binding run over both interpreters
Every unit test and every integration test file of `vkxp-binding` SHALL be built and run a
second time by `vkxp-binding-rgss3` over Ruby 1.9.2, from the same file. A test file of
`vkxp-binding` without its counterpart in the second package SHALL fail a test of the
workspace. The tests of the workspace, and so the gate, SHALL run both sets.

#### Scenario: The same tests
- **WHEN** the tests of both packages are listed with `cargo test --locked -- --list`
- **THEN** every unit test and every test of an integration test file that `vkxp-binding` lists is listed by `vkxp-binding-rgss3`, the latter under one module in front of its name; documentation tests are those of `vkxp-binding` alone

#### Scenario: A test file without its counterpart
- **WHEN** a file is added to `crates/vkxp-binding/tests` and none of that name to `crates/vkxp-binding-rgss3/tests`
- **THEN** a test of `xtask` fails and names the file

#### Scenario: They pass
- **WHEN** `cargo test -p vkxp-binding-rgss3 --locked --no-fail-fast` runs
- **THEN** it passes

#### Scenario: The interpreter of a test
- **WHEN** one test file that both packages build evaluates `RUBY_VERSION` through the harness
- **THEN** the result is `"1.8.7"` in `vkxp-binding` and `"1.9.2"` in `vkxp-binding-rgss3`, each as the constant of its interpreter crate says

### Requirement: Allow-list of tests over Ruby 1.9.2
A test or snippet case that cannot pass over Ruby 1.9.2 yet SHALL be named in
`crates/vkxp-binding-rgss3/tests/ruby192.allow` with a reason that says what differs and
which measurement or later work settles it. A listed test SHALL be run alone in a process
of its own and SHALL count as passed only while it fails there. A listed test or case that
passes SHALL fail the run and be named. The list SHALL have no effect over Ruby 1.8.7.

#### Scenario: A listed test that fails
- **WHEN** a test that is on the list and fails over Ruby 1.9.2 is run by `cargo test`
- **THEN** it is reported as passed, and its output names the reason of its entry

#### Scenario: A listed test that passes
- **WHEN** the part of the harness that decides is given a list that names a test and the outcome that the test passed alone
- **THEN** it fails with a message that names the test and the list file

#### Scenario: A listed test that does not end as a test failure
- **WHEN** the part of the harness that decides is given a list that names a test and the outcome that the process ended by a signal, or that no test ran
- **THEN** it fails and says which of the two happened

#### Scenario: A test taken off the list
- **WHEN** the entry of a test is removed and the marker stays in the test
- **THEN** the body of the test runs in place and its result is the result of the test

#### Scenario: A listed case
- **WHEN** the helper of the snippet cases is given two listed cases over Ruby 1.9.2, one whose outcome differs from its expectation and one whose outcome is as expected
- **THEN** it reports nothing for the first, and for the second a failure that says the case passes and names the list file

#### Scenario: An entry without a reason
- **WHEN** the list holds an entry without a comment line, or a line that names neither a test nor a case
- **THEN** reading it is an error that names the line

#### Scenario: Reasons of the committed list
- **WHEN** an entry of the committed list is read
- **THEN** its reason names a section of a measurement report of RGSS3 that states the other behaviour, or the construct of Ruby 1.8 that Ruby 1.9 has differently, or the symbol numbering of Ruby 1.8.1

#### Scenario: Ruby 1.8.7
- **WHEN** a test with the marker runs in `vkxp-binding`
- **THEN** the list is not read, no process is started and the body runs

### Requirement: Collector stress of the binding over Ruby 1.9.2
`cargo xtask stress` SHALL have a step that runs the tests of `vkxp-binding-rgss3` with the
collector of Ruby 1.9.2 running at every allocation. A test left out of the step SHALL be
named in `xtask/src/stress.skip` with its reason, and a value lost under stress SHALL NOT
be such a reason. The step SHALL NOT be a step of the gate.

#### Scenario: The step
- **WHEN** the steps of the stress command are listed
- **THEN** one of them, `binding-1.9.2`, runs `cargo test -p vkxp-binding-rgss3 --locked --no-fail-fast` with `VKXP_GC_STRESS` set to 1

#### Scenario: A test left out
- **WHEN** the skip list holds a line `binding-1.9.2` with the name of a test
- **THEN** the step skips exactly that test, and no other step does

#### Scenario: The run
- **WHEN** the command of the step runs on the development machine
- **THEN** it passes

#### Scenario: Not in the gate
- **WHEN** the steps of the gate are listed
- **THEN** they are the six they were, and none sets `VKXP_GC_STRESS`

### Requirement: A script of RGSS3 flavour runs through the binding
A test of `vkxp-binding-rgss3` SHALL install the RGSS classes for a game of RGSS3 in an
interpreter of Ruby 1.9.2 and evaluate a script that makes a `Bitmap` and a `Sprite`,
changes the sprite inside a `Fiber` that yields, and resumes the Fiber to its end. A frame
extracted from the guest's scene afterwards SHALL show the sprite where the Fiber left it.
The test SHALL need no window, no guest process and no file of an original runtime.

#### Scenario: The frame
- **WHEN** the script fills a bitmap of 4 by 4 with opaque red, shows it with a sprite, sets `x` to 10 in a Fiber, yields, sets `y` to 20, and the Fiber is resumed twice
- **THEN** the frame drawn by the reference compositor is red at (10, 20) and not red at (9, 20) and at (10, 19)

#### Scenario: Between the two resumes
- **WHEN** a frame is extracted after the first resume of that Fiber
- **THEN** it is red at (10, 0) and not red at (10, 20)

#### Scenario: The language of the script
- **WHEN** the script evaluates `[RUBY_VERSION, "é".encoding.name, ->(n) { n + 1 }.(1), { a: 1 }.keys, Fiber.new { Fiber.yield 1; 2 }.resume]`
- **THEN** the result is `["1.9.2", "UTF-8", 2, [:a], 1]`

#### Scenario: Under collector stress
- **WHEN** the script of the first scenario runs between `GC.stress = true` and `GC.stress = false`
- **THEN** the frame has the same three pixels

### Requirement: XP and VX are unchanged by the second package
Over Ruby 1.8.7 nothing SHALL change: every test of the workspace that passed before SHALL
pass under its name with its expected values unedited. An existing test file of
`vkxp-binding` SHALL be edited only for the marker of the allow-list, the common
constructor of the options in place of the symbol table, the path of one import of a
support module, and the allow-list in the helper of the snippet cases.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` is compared before and after for `vkxp-binding`, `vkxp-ruby-sys`, `vkxp-guest-main` and `vkxp`
- **THEN** every name listed before is listed after

#### Scenario: Expected values
- **WHEN** the difference of `crates/vkxp-binding/tests` against `main` is read
- **THEN** it holds only the four kinds of edit, and no expected value, snippet or test name differs

#### Scenario: The program
- **WHEN** the difference of `crates/vkxp`, `crates/vkxp-guest`, `crates/vkxp-host`, `crates/vkxp-proto`, `crates/vkxp-core` and `third_party` against `main` is taken
- **THEN** it is empty
