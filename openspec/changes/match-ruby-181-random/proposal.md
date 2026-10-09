# Proposal

## Why

The first comparison with the original runtime shows the test game's battle taking a different
course in vkxp although both runs were seeded with the same `srand` value. The original embeds
Ruby 1.8.1; vkxp embeds 1.8.7, whose `srand` initialises the generator differently and whose
`rand(n)` derives an integer from the generator differently. Games are deterministic functions
of `rand`, so replays, comparisons and anything a game derives from a seed require the 1.8.1
sequence.

## What Changes

- The vendored interpreter's `srand` and `rand` are patched to produce, for the same seed, the
  values Ruby 1.8.1 produces: the generator is initialised from the seed as one 32-bit integer,
  and `rand(n)` for an integer `n` is the generator's 53-bit real times `n`, truncated.
- The patch is recorded like the existing one (patch file, `PATCHES.md`, manifest hash).
- The upstream tests that assert the 1.8.7 sequences are adjusted and the change is recorded.

Out of scope: any other behavioural difference between 1.8.1 and 1.8.7.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the random number sequence.

## Impact

- `third_party/ruby-1.8.7/random.c` with its patch record, and `crates/vkxp-ruby-sys` tests.
