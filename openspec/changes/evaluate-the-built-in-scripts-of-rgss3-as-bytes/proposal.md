## Why

On 2026-10-14 `cargo xtask oracle` ran In Search of Immortality (VX Ace, `Game.rgss3a`, DLL
3.0.1.1) along a schedule that saves to file 1 through the game's own menu. The original and
vkxp wrote `Save01.rvdata2` and the game's `AutoSave.rvdata2` at the same frame; vkxp's were
5 bytes longer, and the one difference was the empty name of `@bgs_on_save`, an `RPG::BGS`
made by its default arguments: the original dumps that `""` without an encoding
(ASCII-8BIT), vkxp dumped it with `E` true (UTF-8). The default comes from the built-in
scripts of the DLL, which the original evaluates from a string of bytes, so their literals
are ASCII-8BIT; vkxp compiled them as UTF-8, the encoding of game scripts.

## What Changes

- The built-in scripts file is evaluated from a string of bytes: its literals are
  ASCII-8BIT in RGSS3. Game scripts stay UTF-8. Over Ruby 1.8.7 nothing changes.
- A test holds the two encodings and the dump of a literal of the built-in scripts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the source encoding of the built-in scripts of RGSS3.

## Impact

`crates/vkxp-binding/src/runtime.rs` (one call), a new test file of
`crates/vkxp-binding-rgss3`.
