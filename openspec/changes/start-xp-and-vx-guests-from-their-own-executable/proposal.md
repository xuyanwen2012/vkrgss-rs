# Start the guests of XP and VX from their own executable

## Why

`docs/decisions.md` ("Platforms", macOS, and "Order of work", item 4): the guest of RGSS1
and RGSS2 becomes a program of its own instead of the host executing itself again, so that
no window framework is linked into a process that `fork` copies. Today `vkrgss` starts
itself in the guest role for a game of XP or VX; the guest process then carries everything
`vkrgss` links, `winit` included, which on macOS is AppKit, and save states copy that
process with `fork`. The guest of VX Ace already has its own executable,
`vkrgss-guest-rgss3`, that links nothing of the host.

## What Changes

- A new package and binary `vkrgss-guest-rgss12`, the counterpart of `vkrgss-guest-rgss3`
  over Ruby 1.8.7: it depends on `vkrgss-guest-main` only and links nothing of the host.
- `vkrgss` starts `vkrgss-guest-rgss12` beside it as the guest process of a game of XP or VX,
  as it starts `vkrgss-guest-rgss3` for VX Ace; it no longer starts itself as a guest.
  `--guest thread` stays as it is.
- `cargo xtask dist` puts three executables into the package.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the executable of the guest process.
- `workspace-structure`: the guest executable of RGSS1 and RGSS2; the package holds three
  executables.

## Impact

New `crates/vkrgss-guest-rgss12`; `crates/vkrgss` (`process.rs`, `main.rs`, the hidden guest
role of `vkrgss` itself and its tests), `Cargo.toml` (members), `Cargo.lock`, `xtask`
(`dist`, the dependency checks that exist for `vkrgss-guest-rgss3`), `docs/running.md`,
`docs/building.md`, `docs/development.md` where they name the executables, `CHANGELOG.md`.
No change to what a game sees.
