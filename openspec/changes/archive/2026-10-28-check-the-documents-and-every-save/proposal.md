# Check the documents and every save

## Why

The whole-project reviews of 2026-10-26 (`target/review-2026-10-26/`, Codex defect 10 and
both sections 5) found:

- `cargo xtask xarch` takes a file for a save only when its name contains "save". The saves of
  VX and VX Ace games that write `Save/1.rvdata` or `1.cdat` are never compared, so "every save
  is identical" can hold with no save compared at all.
- About twenty statements in `docs/` and in doc comments disagree with the code: the crate
  table and the list of modules allowed `unsafe` in `docs/architecture.md`, the targets of
  `vkrgss-ruby192-sys`, "no `cfg` tells the interpreters apart", the transitions and
  brightness the binding sends, a `VKRGSS` module that does not exist, a bundled SoundFont that
  is not bundled, an encoding detection that is a fixed order, the slots of bitmaps, the hard
  requirements on the device, and decisions still marked open although they are built.
- Nothing keeps the mechanical ones (crate table, `unsafe` list, target lists) in line.

## What Changes

- `xarch` compares every file the run wrote below the copy of the game, reports per game how
  many saves it compared, and fails when a game declares saves in `xarch.toml` and fewer were
  compared.
- The documents and doc comments are brought in line with the code as it is after the other
  changes of the review round (this change runs last).
- A test of `xtask` checks the crate table of `docs/architecture.md` against `cargo metadata`,
  its `unsafe` paragraph against the `forbid`/`allow(unsafe_code)` attributes, and the target
  lists stated for the interpreter crates against their `targets.rs`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: every save of a comparison game is compared.
- `workspace-structure`: the architecture document is checked against the workspace.

## Impact

`xtask/src/xarch.rs`, `xtask/xarch.example.toml`, `xtask/tests/` (new document test),
`docs/architecture.md`, `docs/decisions.md`, `docs/running.md`, `docs/development.md`,
`docs/compatibility.md`, `README.md` if it repeats a corrected statement; doc comments only in
`vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-guest-rgss3`, `vkrgss-host`, `vkrgss-core`,
`vkrgss-vk`, `vkrgss-image`, `vkrgss-binding`, `vkrgss` (`args.rs` USAGE and the doc of
`GuestForm`'s default). `CHANGELOG.md`. No behaviour of the runtime changes.
