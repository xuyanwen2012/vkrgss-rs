# Confine loading and process functions

## Why

The review of 2026-10-26 (`target/review-2026-10-26/merged.md`, findings C2, F2, F3 and the
signal part of B2) found gaps in what the prelude switches off and in how its `require` and
`load` behave:

- Over Ruby 1.8.7 `Kernel#open("|cmd")` starts a program: only the prelude of Ruby 1.9
  wraps `open`. Over both interpreters `IO.read`, `IO.foreach` and `IO.readlines` (and
  `IO.binread` over 1.9.2) open a command for a name that starts with `|`. On Linux the
  seccomp filter refuses `execve`, so this matters where the guest is not confined: macOS,
  `--sandbox off` and a guest on a thread, where `docs/running.md` says the process functions
  "do nothing".
- `Process.kill` is not switched off, so a script can signal the program and any process of
  the user wherever the guest is not confined (the change `verify-guests-and-deny-signals`
  closes it in the kernel on Linux and Android).
- Over Ruby 1.8.7 `autoload` loads its file with the interpreter's own `require`, which takes
  any absolute path: `autoload :X, "/home/user/x.rb"; X` loads a file outside the game
  directory, against the requirement "Loading code". Only RGSS3 routes `autoload` through the
  rule of `require`.
- `require` adds the file to `$"` before it evaluates it and never takes it out: a file that
  raises while it loads cannot be required again, where both interpreters' own `require`
  record a feature only after it loaded. `load(name, true)` ignores `wrap` and defines
  everything at the top level.

## What Changes

- The process functions that are off also cover `Kernel#open`, `IO.read`, `IO.foreach`,
  `IO.readlines` (and `IO.binread` where it exists) with a name that starts with `|`, and
  `Process.kill`, over both interpreters; each starts or signals nothing, returns `nil` and
  says so once on the error stream. With `--allow-process-functions` they work as Ruby has them.
- `autoload` over Ruby 1.8.7 follows the rule of `require`, as it does in RGSS3.
- `require` records a file only once it has loaded; a file that raises can be required again,
  and a file that requires itself while it loads is not evaluated twice. `load(name, true)`
  evaluates the file in an anonymous module.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the process functions that are off, loading code.

## Impact

`crates/vkrgss-binding` (`prelude_process_off.rb`, `prelude_process_off_ruby19.rb`,
`prelude_shared.rb`, `prelude_load_rgss3.rb`, the tests in `tests/`), the tests of
`crates/vkrgss-binding-rgss3`, `docs/running.md` (the option `--allow-process-functions`, the
process functions), `docs/decisions.md` (the Ruby section where it lists them),
`CHANGELOG.md`. No picture changes.
