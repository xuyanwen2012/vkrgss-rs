## Why

VX Ace is being started (`docs/decisions.md`, "VX Ace"; change `plan-vx-ace`, decision 10:
runtime detection with the archive `Game.rgss3a` and the built-in scripts can start at once,
beside the interpreter work). Today a game of RGSS3 is recognised and then refused before
anything else is looked at: its archive cannot be read (`Game.rgss3a` is RGSSAD version 3,
another table and key scheme than `Game.rgssad` and `Game.rgss2a`), its built-in scripts
cannot be extracted (two of the three blocks start with other text than `module RPG;`), and
nothing has been tried of what the program does for a game before a guest starts. The
changes that add the guest of RGSS3 should find all of that done and tested.

## What Changes

- `vkxp-vfs` reads `Game.rgss3a`, RGSSAD version 3, as the archive of a game of RGSS3: the
  layout measured on two archives of the original on 2026-10-13. Entries are found and read
  like those of the other two archives.
- The run-time package of a game of RGSS3 is the one value `RTP` (usually `RPGVXAce`),
  resolved like that of RGSS2; this is stated and tested.
- `cargo xtask extract-rpg` also writes the blocks that start with `def rgss_main` and
  `def rgss_stop`, so that the image of an RGSS3 DLL gives its three blocks. What it writes
  for a DLL of RGSS1 or an image of RGSS2 does not change.
- The built-in scripts file of RGSS3 is `rpg_builtin_rgss3.rb` at the default location, or
  the file of `--builtin-scripts`; a missing one is an error that names it and the document.
- A run of a game of RGSS3 still ends with the message that VX Ace is planned and not
  supported, with the status it has today, but only after the program has done what it does
  for any game before a guest starts: the run-time package, the built-in scripts file, the
  archive, the script archive that `Scripts=` names. A failure of one of these is reported
  instead of the message.
- A game of RGSS3 does not run after this change. Detection itself does not change: the
  rules of `Library=` and `Scripts=` already give RGSS3, and they stay as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-files`: the archive of RGSS3 and its layout; damaged and arbitrary archives of that
  version; the run-time package of RGSS3.
- `game-launch`: the built-in scripts file of RGSS3; the blocks the extraction tool finds;
  the archive of a game of RGSS3; what a run of RGSS3 does before it is refused.

## Impact

- `crates/vkxp-vfs` (`archive.rs`, `runtime.rs`, `files.rs`, `error.rs`, `lib.rs`, and tests
  in `rtp.rs`).
- `crates/vkxp`: the functions that settle what a guest is set up with (`runtime_of`,
  `guest_inputs`, `builtin_scripts`, `archive`, `run`, `run_guest`; in `main.rs` today, or
  in the module the change `move-the-guest-role-into-a-library` moves them to), the usage
  text in `args.rs`, `tests/end_to_end.rs`.
- `xtask/src/extract.rs`, and the lines of `xtask/src/main.rs` that describe the command.
- `docs/running.md`.
- Not touched: `xtask/src/oracle` and `xtask/src/rgssad.rs` (the oracle on a game of VX Ace
  is a change of its own), `vkxp-binding`, `vkxp-ruby-sys`, `third_party`, `vkxp-core`, the
  protocol, the host, the compositor, `README.md`, `docs/compatibility.md` and
  `docs/decisions.md` (VX Ace stays "planned").
