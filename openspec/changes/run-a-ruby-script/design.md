# Design

## Context

- `crates/vkrgss/src/args.rs` parses the options; `--preload` already evaluates a Ruby file before
  a game's scripts, `--runtime` names the runtime, `--sandbox` and `--allow-process-functions`
  exist.
- The interpreters are started by the guest crates (`vkrgss-guest*`), which also define the RGSS
  bindings and talk to the host over the protocol; confinement (Landlock, seccomp) is applied by
  the guest before the first script.

## Decisions

- **Where it runs.** In the `vkrgss` process itself, on its main thread or a thread with the
  stack size the interpreters need (as the guest thread has), without a host session, window,
  device, audio or protocol. Two interpreters cannot share a process; one run starts one.
- **Which parts to reuse.** The interpreter start-up and the rules of `require` and friends that
  games use, with the script's directory as the root; not the RGSS bindings. Whatever is reused
  for games keeps its behaviour for games (the gate and `xarch check` show it).
- **Streams.** `$stdin`, `$stdout`, `$stderr` are the process's standard streams, unbuffered or
  flushed at exit so that output is never lost; `p` writes to the standard output as `ruby`
  does.
- **Exit status.** As `ruby`: 0 at the end, the given status for `exit`, 1 after an uncaught
  exception with the message and backtrace on the error stream; `SystemExit` and `Interrupt`
  as `ruby` treats them.
- **Confinement.** On Linux the same Landlock rules and seccomp filter as a guest, with the
  script's directory as the directory it may write below; on macOS and elsewhere what the guest
  does there (nothing for now). `--sandbox off` and `--allow-process-functions` as for games.
- **Interface.** `--script FILE` takes every argument after `FILE` as `ARGV`, so it is the last
  option; options that only concern games (`--headless`, `--rtp`, `--dump`, `--frames`, ...)
  together with `--script` are an error that names them.
- **Platforms.** Linux and macOS. Android has no command line for it.

## Out of scope

A standard library, a read-eval-print loop, `-e CODE`, the RGSS classes in script mode.

## Implementation notes

- **A script of RGSS3.** Ruby 1.9.2 cannot be linked into `vkrgss` beside Ruby 1.8.7, so the
  program runs a script of `rgss1` or `rgss2` in its own process and replaces itself with
  `exec` by the guest executable `vkrgss-guest-rgss3` beside it for a script of `rgss3`. The
  process, its standard streams and its status stay those of the program. The settled
  options reach that executable as the hidden option `--script-role` of `vkrgss-guest`
  (runtime, sandbox, process functions, verbosity, file, arguments) and the level of the
  diagnostics as `VKRGSS_LOG`; `main` of `vkrgss-guest-rgss3` asks for that option before the
  guest role. This touches the guest executable of RGSS3, a guest start-up crate, by a few
  lines.
- **What is shared.** `run_script` of `vkrgss-guest-main` starts the interpreter as for a game
  of the runtime and runs the script on its thread through `run_confined` of `vkrgss-guest`,
  which `play_in_process` now calls for a game. `run_script` of the binding evaluates the
  parts of the prelude of a game that load files and turn the process functions off, over
  the host functions `vkrgss_locate` and `vkrgss_relative`, whose bodies the game's versions
  and the script's share. The loading rules of RGSS3 moved out of `prelude_rgss3.rb` into
  `prelude_load_rgss3.rb`, which a game evaluates right after it, from the line they stood
  on, so that the positions of its backtraces stay as they were; `xarch check` shows the
  games unchanged.
- **Working directory.** A script runs in the directory it was started from, as with
  `ruby`; only `require` and its kin take the script's directory as their root.

## Follow-ups

- **Interrupt.** Ctrl-C ends a script by the default action of SIGINT, as the interpreter
  crates put the signal dispositions of the process back after `ruby_init`: the shell sees
  the same signal as after `ruby`, but no `Interrupt` is raised, so neither `ensure` clauses
  nor `at_exit` blocks run and no report is written. Raising `Interrupt` needs Ruby's own
  handler of SIGINT kept, and the signal delivered to the interpreter thread, which is a
  change of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys`.
- **Positions in a report.** The positions of the script itself are its absolute path, which
  `require_relative` needs to find its directory; `ruby` writes the path as it was given.
