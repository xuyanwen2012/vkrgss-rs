# Confine the guest process

## Why

A game is a program: its scripts run in the embedded Ruby 1.8.7 with the rights of the user
who starts it. The functions that start processes are switched off in Ruby, but a script can
still write any file the user can write, and a defect in the interpreter's C code that a
crafted save file or script reaches is not held back by a switch written in Ruby.
`docs/decisions.md` names an OS-level sandbox for the guest process as still to come; the
guest has run in a process of its own since `run-the-guest-in-a-process`.

## What Changes

- A guest process restricts itself with Landlock before the first script runs: it can read
  files, write only below the game directory, execute nothing and open no TCP connection.
- `--sandbox on|off`, `VKXP_SANDBOX` and the configuration key `sandbox` choose; the default
  is on. Where the kernel cannot do it, the run says so once and goes on.
- With `--allow-process-functions` the guest is not confined.
- A guest on a thread is not confined; the documents say so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the confinement and its option.

## Impact

`crates/vkxp` (the guest's start, arguments, configuration, tests), `Cargo.toml` and
`Cargo.lock` (the `landlock` crate), `deny.toml` if its licence needs listing,
`docs/running.md`, `docs/decisions.md` (the sentence that the sandbox is still to come),
`README.md` where it lists the options. No picture changes.
