# Proposal

## Why

A game stopped with a `SyntaxError` on a regular expression that the original runtime accepts:
`/\[Vv][([0-9]+)]/`, a character class that contains another one. The original embeds Ruby
1.8.1 built with the Oniguruma regular expression engine; vkxp embeds Ruby 1.8.7 with that
version's own engine. A probe of 65 patterns in both runtimes gives different results for 17:
nested character classes and class intersection, named groups, look-behind, possessive
quantifiers, multi-byte characters in classes, the wording of errors. Scripts written for RGSS
use these freely, so the engine has to be the one the original has.

## What Changes

- The vendored interpreter's regular expression engine is replaced by Oniguruma 5.9.6, built
  from vendored source through its GNU-compatible interface, with Ruby syntax and the encoding
  that `$KCODE` selects.
- The interpreter's own `regex.c` is no longer compiled.
- Tests pin the results measured on the original.

Out of scope: differences between Oniguruma 5.9.6 and the older Oniguruma inside the original
that the probe does not show.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the regular expression engine.

## Impact

- `third_party/` (a new vendored tree with its manifest), `third_party/ruby-1.8.7/re.c` with
  its patch record, `crates/vkxp-ruby-sys` (build script, tests), `LICENSE` notices.
