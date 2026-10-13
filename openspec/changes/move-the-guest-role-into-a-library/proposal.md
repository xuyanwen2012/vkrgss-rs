# Move the guest role into a library

## Why

A game of RPG Maker VX Ace will run in a guest executable of its own, `vkxp-guest-rgss3`,
because Ruby 1.9.2 cannot be linked into the executable that holds Ruby 1.8.7 (change
`plan-vx-ace`, decisions 2 and 10). That executable has to do what the guest process of
`vkxp` does today: take the hidden option that names the socket, attach, copy itself for
states, confine itself. Today this lives in the program crate (`copier.rs`, `sandbox.rs`,
`signals.rs`, and `run_guest` in `main.rs`), which also links the window, Vulkan and audio,
and `copier.rs` calls a C function that only Ruby 1.8.7 has. The move is independent of
Ruby 1.9.2 and can be done, and tested on XP and VX, before the second executable exists.

## What Changes

- A new library crate, `vkxp-guest`, holds the guest role: the hidden option, the
  attachment, the copier of states, the restriction of the process and the signal settings
  of a guest. It links no interpreter and no binding: the executable gives it the function
  that runs the game.
- `vkxp` keeps the host half (starting, attaching, relaying, ending guest processes) and
  calls the library for its own guest, as a thread and as a process, for XP and VX.
- What the guest of `vkxp` needs of the binding and the interpreter is gathered in one
  module of the program that names no host type.
- A guest process states the version of its executable when it connects, and the program
  refuses one of another version with an error that names both. Today both are one file,
  so this never fails; it is there for the second executable.
- The program asks one function, from the runtime of the game, for the executable it starts
  as the guest. Today the answer is always the program itself.
- Nothing else changes: frames, saves, states, the sandbox, the exit status and the messages
  on the error stream are as before, and no message of the protocol changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the crate `vkxp-guest`, its place in the dependency rules, and what
  it may not depend on.
- `game-launch`: the version a guest process states and the executable the program starts.

## Impact

- New: `crates/vkxp-guest`.
- `crates/vkxp` (`main.rs`, `process.rs`, `args.rs`, a new module for the guest's glue;
  `copier.rs`, `sandbox.rs` and `signals.rs` leave it; the dependency on `landlock` goes).
- `crates/vkxp-ruby-sys`: one function, which restarts the timer of the interpreter's
  threads in a copy of the process.
- `xtask/src/deps.rs`: `vkxp-guest` among the guest crates, and one new rule.
- `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `docs/architecture.md`, `docs/decisions.md`,
  `docs/running.md`.
- Not touched: `vkxp-binding`, `vkxp-proto`, `vkxp-host`, the end-to-end tests that exist.
