# Proposal

## Why

The pieces a Ruby script needs to exist are there, each on its own: the interpreter
(`vkxp-ruby-sys`), the value types and bitmaps (`vkxp-core`), and file lookup and image decoding
(`vkxp-vfs`, `vkxp-image`). Nothing connects them, so a script cannot yet
say `Bitmap.new("Graphics/Titles/001-Title01")` or `Color.new(255, 0, 0)`. This change builds
the binding's foundation and the first classes on it.

## What Changes

- `vkxp-ruby-sys` gains registration of host functions with a fixed number of arguments, so
  that methods report the arities of the original runtime and Ruby itself rejects a wrong
  argument count.
- `vkxp-binding` gains its foundation: the guest state that Ruby classes operate on (bitmap
  arena, font defaults, game files, text backend), the way Ruby objects wrap Rust values and
  handles, argument checking, and the mapping from Rust errors to the Ruby exceptions of the
  original runtime (`RGSSError`, `Errno::ENOENT`, `TypeError`, `ArgumentError`).
- The Ruby classes `Color`, `Tone`, `Rect`, `Table` and `Font` with every method of the original
  runtime at the original arity, including `_dump` and `_load` so `Marshal` reads and writes the
  original data files.
- The Ruby class `Bitmap`: creation from a size or from a file found through the game's files
  and the RTP, every drawing method, text through the text backend the guest is configured
  with, copying, disposal.
- A test harness that boots the interpreter with this binding and evaluates Ruby, used by this
  and later binding changes.

Out of scope: `Graphics`, `Input`, `Audio`, sprites, viewports, windows, planes, tilemaps,
loading and running a game's scripts, the built-in `RPG` module, and the text backend over
`vkxp-text` (real fonts are wired in by the change that runs games; tests here use core's
deterministic rasteriser).

## Capabilities

### New Capabilities

- `rgss-values`: the Ruby classes `Color`, `Tone`, `Rect`, `Table` and `Font`.
- `rgss-bitmap`: the Ruby class `Bitmap`.

### Modified Capabilities

None.

## Impact

- Crate `vkxp-ruby-sys`: one addition to its definition helpers (fixed-arity methods).
- Crate `vkxp-binding`: new modules and tests; dependencies on `vkxp-ruby-sys`, `vkxp-core`,
  `vkxp-vfs`, `vkxp-image` and `vkxp-proto`. `unsafe` is allowed in this crate for the calls
  into the interpreter.
- A data file with the original runtime's class, method and arity list is committed with the
  tests.
- Later binding changes (runtime and modules; scene classes) build on the foundation here.
