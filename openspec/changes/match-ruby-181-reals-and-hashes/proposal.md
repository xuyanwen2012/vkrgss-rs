# Proposal

## Why

A probe script run in the original runtime (Ruby 1.8.1) and in vkxp shows three differences
that make games take another course than in the original:

- `rand`: the original builds each real number from one 32-bit output of the generator; the
  earlier change `match-ruby-181-random` assumed two (53 bits), so vkxp consumes the sequence
  twice as fast.
- `String#hash`: 1.8.7 uses another function, seeded randomly at start-up, so the iteration
  order of hashes with string keys differs from the original and even from run to run.
- `Float#hash` differs as well.

## What Changes

- The vendored interpreter's real number for `rand` is one 32-bit output divided by 2^32.
- `String#hash` and `Float#hash` are the 1.8.1 functions, computed in 32-bit arithmetic, with
  no random seed.
- Each patch is recorded like the existing ones. Tests use values measured on the original.

Out of scope: the numbering of symbols (a hash with symbol keys can still iterate in another
order than in the original), every other difference between 1.8.1 and 1.8.7.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: random reals and hash values.

## Impact

- `third_party/ruby-1.8.7/random.c`, `string.c`, `numeric.c` with their patch records, and
  tests of `crates/vkxp-ruby-sys`. Upstream tests that pin other values are adjusted.
