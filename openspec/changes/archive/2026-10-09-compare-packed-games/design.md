# Design

## Context

- `xtask/src/oracle` copies the game, reads `Data\Scripts.rxdata` (the name `Game.ini` gives),
  adds the probe as the first script and writes it back, per side.
- RGSSAD version 1: the eight bytes `RGSSAD\0\x01`, then entries until the end of the file. A
  32-bit key starts at `0xDEADCAFE` and is advanced by `key = key * 7 + 3`. Each entry: name
  length XOR key (key advanced); name bytes, each XOR the key's low byte (key advanced after
  every byte); data size XOR key (key advanced); data, XORed word by word (little endian) with
  a copy of the key as it stands after the size, that copy advanced after every word and the
  main key not advanced by the data. Names use backslashes and are UTF-8.
- Measured on the original runtime: an archive written this way from the test game's `Data`
  and `Graphics` folders, with the folders removed, is read and played; a script archive left
  loose next to an archive is not found; graphics left loose are found.
- `vkxp-vfs` has a reader for the format; `xtask` does not depend on runtime crates.

## Decisions

- A small reader and writer of the format in `xtask`, independent of `vkxp-vfs`, so that the
  harness and the runtime check each other. Entries are kept in file order; only the script
  archive's entry is replaced.
- The lookup of the script archive's entry compares names without regard to case and with
  either separator.
- `pack-rgssad` works in place on the directory it is given, which is meant to be a copy.
- The coordinator runs the original side on a packed copy of a real game after the merge.
