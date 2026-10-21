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

## Follow-ups
