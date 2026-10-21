# Run a Ruby script

## Why

The interpreters vkrgss embeds (Ruby 1.8.7 and 1.9.2) are complete, and their own test suites
pass in the gate, but the only way to run Ruby code is as part of a game: a game directory, its
`Scripts` file, and at best `--preload` with an `exit` at its end. Running a plain program, such as
an exercise of algorithms, to see what the interpreter of a runtime does needs none of that.

## What Changes

- `vkrgss --script FILE [ARG...]` runs one Ruby file with the interpreter of a runtime and no
  game: no game directory, no window, no Vulkan device, no audio. `--runtime rgss1|rgss2|rgss3`
  chooses the interpreter (Ruby 1.8.7 for the first two, 1.9.2 for the third); the default is
  `rgss3`. The arguments after the file are `ARGV`.
- The script has the standard input, output and error streams of the terminal; `gets` and
  `STDIN.read` read the standard input. The exit status is that of `exit`/`exit!`, 0 at the end
  of the file, and 1 after an uncaught exception, whose message and backtrace go to the error
  stream as `ruby` writes them.
- `require`, `load`, `require_relative` and `autoload` follow the rule of games with the script's
  directory in place of the game directory. There is no standard library, as for games.
- The modules and classes of RGSS are not defined.
- The process confines itself as a guest does (on Linux, Landlock then the seccomp filter): it
  writes only below the script's directory and starts no program; `--sandbox off` and
  `--allow-process-functions` work as for games.

## Capabilities

### New Capabilities

- `script-mode`: running a Ruby file with the interpreter of a runtime, without a game.

### Modified Capabilities

None.

## Impact

`crates/vkrgss` (arguments, the new mode), the crates that start an interpreter and confine a
guest (reused, not changed in behaviour for games), `docs/running.md`, `CHANGELOG.md`. Games run
as before.
