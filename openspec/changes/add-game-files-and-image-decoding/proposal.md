# Proposal

## Why

A game names its files the way Windows lets it: any letter case, either slash, usually without
an extension, and expecting the run-time package (RTP) to supply whatever the game folder lacks.
The test game ships no title picture, window skin or music of its own; all of those come from
the RTP. Nothing in vkxp can find or decode such a file yet, and the Ruby binding cannot load a
single picture without it.

## What Changes

- `vkxp-vfs`: reads `Game.ini`, resolves the RTP names it lists to directories, and resolves
  script-style paths against the game directory and then the RTPs, ignoring letter case,
  accepting both slashes and trying the extensions of the requested kind (image, audio, data).
- `vkxp-image`: decodes PNG, JPEG and BMP files, recognised by content, into the bitmap pixel
  layout of the protocol (red, green, blue, straight alpha).
- Both report typed errors and never panic on malformed input.

Out of scope: the encrypted archive (`Game.rgssad`), audio decoding, writing files (saves are
written by scripts through Ruby's own file API), bit-exact agreement of JPEG decoding with the
original runtime, and any Ruby-facing API.

## Capabilities

### New Capabilities

- `game-files`: finding a game's files: the game configuration file, RTP directories, and the
  resolution of script-style paths to real files.
- `image-decoding`: turning PNG, JPEG and BMP file contents into bitmap pixels.

### Modified Capabilities

None.

## Impact

- Crates: `vkxp-vfs`, `vkxp-image`. Both are guest crates and stay free of `unsafe`.
- New dependencies: `encoding_rs` (legacy text encodings of `Game.ini`), `zune-jpeg` (JPEG);
  `png` is already in the workspace. Development-only: an image encoder to synthesise test
  inputs.
- `Cargo.toml` (workspace dependency table) and `Cargo.lock` change accordingly.
