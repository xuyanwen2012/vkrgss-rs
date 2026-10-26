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

## Notes of the implementation

- **Callers of the pipe form** (task 1.1). Ruby 1.8.7 `io.c`: `pipe_open` is called by
  `rb_io_s_popen` (`IO.popen`), `rb_f_backquote` (backticks) and `rb_io_open`, and
  `rb_f_open` calls `rb_io_s_popen` itself for a name that starts with `|`; `rb_io_open` is
  called only by `rb_io_s_foreach`, `rb_io_s_readlines` and `rb_io_s_read`. Ruby 1.9.2 `io.c`:
  `check_pipe_command` is called only by `rb_f_open` and `rb_io_open`; `rb_io_open` only by
  `open_key_args` (`IO.foreach`, `IO.readlines`, `IO.read`) and `rb_io_s_binread`. The
  `open_args:` key of 1.9.2 opens through `rb_io_open_with_args`, which is `File.new` and
  takes no command; `IO.copy_stream` opens a name with `File.new`, `ARGF` with `rb_fopen`
  (1.8.7) or `rb_sysopen` (1.9.2), `IO#reopen` with `rb_sysopen` or `freopen`. No other
  caller starts a program, so the stubs of the design cover every one.
- **The name is converted once.** The stubs convert the first argument themselves and pass
  the converted String on, so that the name checked is the name the original opens: an
  object whose `to_str` answers a file name at the check and a command at the open would
  otherwise pass. Ruby 1.8.7 takes the name with `to_str`; Ruby 1.9.2 takes it with `to_path`
  before `to_str`, and its `open` gives an object that responds to `to_open` to that method
  (`rb_f_open`, `rb_get_path_check`). `vkrgss_file_name` in `prelude_process_off.rb` has the
  rule of 1.8.7 and `prelude_process_off_ruby19.rb` replaces it with that of 1.9.2, so
  `File.read(object)` with `to_path` answering `"|cmd"` is refused over 1.9.2 too.
- **`Process.kill`** is a module function in both interpreters (`rb_define_module_function`),
  unlike `Process.fork`, which is a singleton method only, so the stub is a module function
  too and the copy a script gets with `include Process` signals nothing either. Its line on
  the error stream says "nothing was signalled" in place of "nothing was started".
- **`require` and `load`** (task 2.1). `prelude_shared.rb` defines `vkrgss_require`, which
  keeps `$vkrgss_loading`, and `vkrgss_load`, after the definitions whose lines it keeps;
  the `require` and `load` of `prelude_load_rgss3.rb` call them with their UTF-8 source, so
  every definition of both files stands on the line it stood on. A file that requires
  itself gets `false` from the inner `require`, as from the interpreters' own.
- **Difference of the wrapped load.** `rb_load` with `wrap` evaluates the file with `self`
  a copy of `main` extended with the anonymous module, so the file can call a method it
  defined with `def` at its top level. `Module.new.module_eval` evaluates it with `self` the
  module, of which such a method is an instance method: a call of it from the file's own top
  level raises `NoMethodError`, and `self` is the module, not `main`. Constants and classes
  stay in the module in both.

## Follow-ups

- **`Process.exec` over Ruby 1.8.7 is on.** Ruby 1.8.7 has `Process.exec` too (`process.c`,
  a singleton method of `Process`), and only the prelude of Ruby 1.9 stubs it. The
  requirement names `exec`, which the stub of `Kernel#exec` covers, and the test
  `a_confined_guest_cannot_start_a_program` of `crates/vkrgss/tests/end_to_end.rs` uses
  `Process.exec` of Ruby 1.8.7 on purpose to reach `execve` and see the seccomp filter
  refuse it. Stubbing it over 1.8.7 needs another way for that test to reach `execve`
  (outside the scope of this change); where the guest is not confined it starts a program.
- **The originals stay reachable.** The stubs keep the originals under private aliases
  (`vkrgss_open`, `vkrgss_read`, `vkrgss_foreach`, `vkrgss_readlines`, `vkrgss_binread`), as
  the design asks, and a script reaches a private method with `send`:
  `send(:vkrgss_open, "|cmd")` starts a program where the guest is not confined. Keeping
  them out of reach would hold each original as an `UnboundMethod` in the closure of a
  `define_method` in place of an alias (both interpreters take `|*args, &block|` there,
  probed over Ruby 1.8.7 on 2026-10-26); the design keeps the alias, so this is left for a
  decision.
