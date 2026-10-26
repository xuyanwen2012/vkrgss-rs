# Design

## Context

- `crates/vkrgss-binding/src/prelude_process_off.rb` stubs `system`, `exec`, `fork`,
  `syscall`, backticks, `Process.fork` and `IO.popen`; `prelude_process_off_ruby19.rb` adds
  `spawn`, `Process.spawn`, `Process.exec` and `open` with a command over Ruby 1.9.2
  (`runtime.rs` evaluates it when `rb::RUBY_19`).
- Where the pipe form lives: Ruby 1.8.7 `io.c` `rb_f_open` (a name starting with `|`) and
  `rb_io_open`, which `IO.read`, `IO.foreach` and `IO.readlines` use; Ruby 1.9.2
  `check_pipe_command`, reached from `rb_f_open` and `rb_io_open`, which `IO.read`,
  `IO.foreach`, `IO.readlines` and `IO.binread` use. `File` inherits these class methods.
- `prelude_shared.rb` defines `require` and `load` for every runtime; `prelude_load_rgss3.rb`
  replaces them in RGSS3 and also defines `require_relative`, `Kernel#autoload` and
  `Module#autoload` through `vkrgss_locate`. Ruby 1.8.7's own `rb_require_safe` provides the
  feature after the load (`eval.c`), 1.9.2 likewise.
- The prelude keeps the line numbers of some definitions on purpose (comment lines in
  `prelude_shared.rb` and `prelude_rgss3.rb` say so); names the prelude interns are made
  before `mark_game_scripts` and take no number of the original's sequence.

## Decisions

- **One wrapper of the pipe form for both interpreters.** The stub of `Kernel#open` moves from
  `prelude_process_off_ruby19.rb` to `prelude_process_off.rb`, so Ruby 1.8.7 has it too; it
  keeps its alias (`vkrgss_open`) and its rule: a first argument that responds to `to_str` and
  starts with `|` is refused, anything else goes to the original. `IO.read`, `IO.foreach`,
  `IO.readlines` and, where `IO.respond_to?(:binread)`, `IO.binread` get the same rule as
  singleton methods of `IO`, so `File.read("|x")` is covered by inheritance. The stubs report
  with `vkrgss_process_function_off` under the name of the call (`open`, `IO.read`, ...).
  The implementer checks `io.c` of both interpreters for any other caller of the pipe form and
  covers it the same way, or lists it in this file with the reason it cannot start a program.
- **`Process.kill` is a process function.** It is stubbed in `prelude_process_off.rb` like
  `Process.fork`. Games use it rarely; a script that checks its own process with
  `Process.kill(0, Process.pid)` gets `nil` and a line on the error stream.
- **`autoload` over Ruby 1.8.7.** `prelude_shared.rb` gets the `Module#autoload` and
  `Kernel#autoload` of `prelude_load_rgss3.rb` (locate by the rule of `require` at the call,
  register the located path), which RGSS3 keeps replacing with its own. Ruby 1.8.7 accepts an
  absolute path in `autoload`, so the located path loads through the interpreter's loader as
  in RGSS3; the requirement "Loading code in RGSS3" stays as it is.
- **`require` records after the load.** Both `require`s keep a list of files being loaded (a
  global of the prelude, e.g. `$vkrgss_loading`, or a constant hidden in `Kernel`): a file in
  `$"` or in that list returns `false`; otherwise it is put in the list, evaluated, and on
  success appended to `$"`; it leaves the list in an `ensure`. An exception propagates as
  before. This is what the interpreters' own `require` does, without its thread waiting.
- **`load(name, true)`.** The file is evaluated with `Module.new.module_eval(source, path)`,
  so constants and classes it defines stay in the anonymous module. Methods it defines with
  `def` at its top level become methods of that module and are not callable from the top
  level; that is close to `rb_load` with `wrap`, which also extends a copy of `main`. Any
  difference found is written here.
- **Line numbers and symbols.** New code goes after the definitions whose lines are kept, or
  into the files that have no such rule. The tests of symbol numbers (`run_game_symbols`,
  `run_game_first_uses`, `run_game_lazy_names` and the interpreter crates' `symbol_ids*`) must
  pass unedited; a new name of the prelude must not take a number of the original's sequence.

## Out of scope

The kernel side of signals (change `verify-guests-and-deny-signals`); `Game.ini` encodings
(the code does what the requirement "Text encoding of the configuration file" says; only
`docs/decisions.md` calls it a detection, which the documents change fixes); the check-then-use
of symbolic links in `vkrgss-vfs`, which only lets a script read a file it could read anyway,
because reading is not restricted.

## Follow-ups

None yet.
