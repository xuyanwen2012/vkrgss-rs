# Design

## Context

- `vkxp-vfs` resolves a kind (image, audio, data) and a name against the game directory and the
  RTP directories, case-insensitively and with an extension search, and returns bytes.
- RGSSAD version 1: the eight header bytes `RGSSAD\0\x01`, then entries until the end of the
  file. A 32-bit key starts at `0xDEADCAFE`. Each entry is: name length (u32 little endian,
  XOR key, then key = key * 7 + 3); name bytes (each XOR the key's low byte, key advanced after
  every byte); data size (u32 XOR key, key advanced); data. The data is decrypted with a copy of
  the key as it stands after the size: each little-endian 32-bit word XOR that key, the key
  advanced after every word; the main key is not advanced by the data. Names use `\`.
- Names inside archives of Chinese and Japanese games are UTF-8.

## Decisions

- On opening the game's files the archive's entry table is read once into an index from the
  normalised name (lower case, `/`) to offset, size and data key; the file is kept open and
  an entry is decrypted when read.
- Lookup goes through the existing resolution code with the archive as the first root, so the
  extension search and the case rules are shared, not duplicated.
- PLACEHOLDER: the precedence of the archive over loose files follows common knowledge of the
  original and is to be confirmed with the oracle harness.
- Tests build archives in memory with a small encrypting helper kept in the test module.

## Risks / Trade-offs

- [Very large archives] → Only the table is held in memory; entries are read on demand.

## Follow-ups

- The archive is read by a new constructor, `GameFiles::open`, because opening can now fail
  and `GameFiles::new` is infallible and used by other crates. This change is confined to
  `vkxp-vfs`, so the executable (`crates/vkxp/src/main.rs`) still calls `GameFiles::new` and
  does not read archives until it is switched to `GameFiles::open`.
- `GameFiles::resolve` returns the path of a real file, so it never finds an archive entry;
  only `GameFiles::read` does. Its one caller outside the crate is the lookup of `.rb` files
  for `require` and `load`. Whether anything needs a path-less lookup is open.
- An entry whose name is not valid UTF-8 is left out of the index, as loose files with such
  names are. Whether archives with names in a legacy encoding exist is not known.
- Of two entries whose names differ only in letter case the first is kept; loose files prefer
  the exact match. What the original does with such an archive has not been measured.
