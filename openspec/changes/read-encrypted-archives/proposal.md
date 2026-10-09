# Proposal

## Why

Most released RPG Maker XP games ship their `Data` and `Graphics` folders inside an encrypted
archive, `Game.rgssad`, that the original runtime reads transparently. vkxp only reads loose
files, so such games do not start. This is the first step of milestone M6.

## What Changes

- `vkxp-vfs` reads the RGSSAD version 1 archive of a game directory and resolves images, audio
  and data files from it with the same rules as loose files (case-insensitive names, either
  path separator, extension search).
- A file is looked up in the archive first, then in the game directory, then in the RTPs.
- A damaged archive is an error that names the file; a game without an archive behaves as
  before.

Out of scope: writing archives, the version 3 format of later RPG Maker versions, any other
form of protection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-files`: files inside the encrypted archive.

## Impact

- Crate `vkxp-vfs` only.
