# Hand the guest process its settings

## Why

A game of RPG Maker VX Ace will run in a guest executable of its own, `vkxp-guest-rgss3`
(`docs/decisions.md`, "VX Ace"). The change `move-the-guest-role-into-a-library` made the
guest role a library and left two things open that this executable cannot do without:

- The child of `vkxp` gets its settings by parsing the whole command line of the program and
  reading its configuration files again, with a parser that names types of `vkxp-host`. An
  executable that may not link `vkxp-host` cannot use it.
- What a guest does with its binding and its interpreter (`guest.rs`, `font.rs`) is in the
  program crate, where a second executable cannot reach it, and the subscriber that writes
  the diagnostics is in a module that names a type of `vkxp-host`.

Both can be settled, and tested on XP and VX, before Ruby 1.9.2 and the second executable
exist. After this change, adding `vkxp-guest-rgss3` is a binary crate whose `main` is one
call, over the binding and the glue compiled a second time.

## What Changes

- The program writes what a guest process needs (the game directory, the settings a guest
  reads, the seed, the socket, the diagnostic level) into a record, a file in the private
  directory of the socket, and starts the guest with `--guest-role <record>` and no other
  argument. The guest process reads the record and neither the program's command line nor a
  configuration file. `vkxp` does this for its own child too: there is one way.
- The record states the version of the program that wrote it, and a guest executable
  refuses one of another version, one that is missing and one that is malformed, with a
  message that names the file and a non-zero status.
- **BREAKING** for nobody but a user who passed the hidden option by hand: its value is no
  longer `SEED=SOCKET`, and the option is known only as the first argument.
- A new library crate, `vkxp-guest-main`, holds the glue: what a guest is set up with, the
  text backend, the guest on a thread and the function that makes an executable a guest
  process. It is a guest crate. `vkxp` links it, for `--guest thread` and for its own child,
  and calls that function first in `main`.
- `vkxp-guest` gets the record, with the plain record of the settings, and the subscriber
  of the diagnostics. `log.rs` of the program keeps what names the host.
- Nothing else changes: frames, saves, states, the sandbox, the exit status and the lines on
  the error stream of a run of XP or VX are as before, no message of the protocol changes
  and the protocol version stays 12.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: how a guest process gets its settings, and what an executable does with a
  record it cannot use.
- `workspace-structure`: the crate `vkxp-guest-main`, its place among the guest crates, and
  what `vkxp-guest` holds.

## Impact

- New: `crates/vkxp-guest-main` (from `crates/vkxp/src/guest.rs` and `font.rs`, which leave
  the program).
- `crates/vkxp-guest`: new modules for the record, the settings and the diagnostics;
  `role.rs` loses the value `SEED=SOCKET`; dependencies on `vkxp-vfs`, `serde`, `postcard`
  and `tracing-subscriber`.
- `crates/vkxp`: `main.rs`, `process.rs`, `args.rs`, `log.rs`, the manifest; one test added
  to `tests/end_to_end.rs` and a new test file for the refusals.
- `xtask/src/deps.rs`: `vkxp-guest-main` among the guest crates.
- `Cargo.toml`, `Cargo.lock`, `docs/architecture.md`, `docs/decisions.md`,
  `docs/running.md`.
- Not touched: `vkxp-binding`, `vkxp-ruby-sys`, `vkxp-proto`, `vkxp-host`, `vkxp-vfs`,
  `vkxp-text`, and every test that exists in `crates/vkxp/tests/end_to_end.rs`.
