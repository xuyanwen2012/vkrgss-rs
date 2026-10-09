# Proposal

## Why

Symbols that a game's scripts create get other numbers in vkxp than in the original, because
Ruby 1.8.7 and the binding intern a different set of names during start-up than Ruby 1.8.1 and
the original runtime do. Symbol numbers decide the order of instance variables, of hashes with
symbol keys and of everything derived from them. Measured on the test game: a save written at
the same frame holds the same data in both runtimes but differs in bytes, only in the order of
instance variables. Save files should be byte-identical, and scripts that iterate instance
variables or symbol-keyed hashes should see the original's order.

## What Changes

- The vendored interpreter gives every name that the original has at start-up the number it
  has there, from a committed table measured on the original (2002 symbols).
- Names interned before the game's scripts start that are not in the table get numbers from a
  separate high range; from the game's first script on, new names are numbered from the
  original's next free number.
- The binding tells the interpreter where the game's scripts begin.

Out of scope: differences of the table between RGSS DLL versions (the table is from 1.03).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: symbol numbering.

## Impact

- `third_party/ruby-1.8.7/parse.y` and `parse.c` with their patch record and a generated
  table, `crates/vkxp-ruby-sys` (one function), `crates/vkxp-binding` (one call).
