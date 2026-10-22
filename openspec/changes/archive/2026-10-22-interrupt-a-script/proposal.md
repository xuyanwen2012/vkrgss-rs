# Interrupt a script

## Why

In script mode (change `run-a-ruby-script`) Ctrl-C ends a script by the default action of
SIGINT: the interpreter crates put the signal dispositions of the process back after
`ruby_init`, as games need. No `Interrupt` is raised, so `ensure` clauses and `at_exit` blocks
do not run and nothing is reported, where `ruby` raises `Interrupt` in the main thread.

## What Changes

- In script mode only, SIGINT raises `Interrupt` in the script, as in `ruby` (1.8.7 and 1.9.2):
  a script can rescue it; uncaught, `ensure` clauses and `at_exit` blocks run, the report goes to
  the error stream, and the process then ends by SIGINT, so the shell sees the same status as
  after `ruby` (130 in a shell).
- Games are unchanged: the host's signal handlers are restored after `ruby_init` as now.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `script-mode`: a requirement on SIGINT.

## Impact

`crates/vkrgss` (script mode), `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` where the restoring of
the dispositions after `ruby_init` gains a way to keep Ruby's handler of SIGINT, the guest crates
if script mode starts the interpreter through them, `docs/running.md`, `CHANGELOG.md`.
