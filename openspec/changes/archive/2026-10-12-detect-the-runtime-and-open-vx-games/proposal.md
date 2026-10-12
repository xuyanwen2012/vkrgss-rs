## Why

The scope now covers RPG Maker VX (`docs/decisions.md`, "Several runtimes"), and milestone V0
has passed. Nothing in the program knows yet which runtime a game belongs to: it reads
`RTP1` to `RTP3`, looks for `Game.rgssad`, loads one built-in scripts file and sets `$DEBUG`.
A VX game names its run-time package with `RTP=`, keeps its data in `Game.rgss2a`, needs the
built-in scripts of RGSS2 and reads `$TEST`. The decisions say that these are settled before
V1.

## What Changes

- A runtime value (RGSS1, RGSS2, RGSS3) and its detection from `Library=` and the extension
  of the file `Scripts=` names, with an error when the two disagree or say nothing.
- The option `--runtime` decides when given.
- By runtime: the key of the run-time package (`RTP=` for RGSS2), the archive
  (`Game.rgss2a`), the default built-in scripts file, and the globals of test mode
  (`$TEST` for RGSS2, measured on the original).
- A VX game is run as far as the code goes and the run says once that VX is experimental. A
  VX Ace game ends at once with a message that says it is planned.
- XP behaves as before in every case.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-files`: the runtime of a game; `RTP=`; the archive by runtime.
- `game-launch`: the option; the built-in scripts file by runtime; what a run of VX and of
  VX Ace says.
- `rgss-runtime`: the interpreter's globals by runtime.

## Impact

- `crates/vkxp-vfs` (configuration file, run-time packages, archive name, the runtime value),
  `crates/vkxp` (option, built-in scripts file, messages), `crates/vkxp-binding`
  (`runtime.rs`: the globals; the test host's generated games).
- `docs/running.md`.
- Not touched: `crates/vkxp-ruby-sys` and `third_party` (the symbol table by runtime is the
  change `number-symbols-by-runtime`), `xtask` (the oracle is `run-the-oracle-on-vx-games`),
  the protocol, the host, the compositor.
