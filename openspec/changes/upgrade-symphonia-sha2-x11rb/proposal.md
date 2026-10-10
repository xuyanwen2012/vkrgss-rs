# Proposal

## Why

Three direct dependencies have a newer major (pre-1.0 minor) release than the one the
workspace allows: `symphonia` 0.5 (0.6.1 is current), `sha2` 0.10 (0.11.0) and `x11rb` 0.13
(0.14.0). Every other direct dependency is at its newest stable release. Staying behind means
fixes land only in lines we do not use, and the distance grows.

## What Changes

- `symphonia` moves to 0.6 in `vkxp-audio`, `sha2` to 0.11 in the tests of `vkxp-ruby-sys`,
  `x11rb` to 0.14 in `xtask`. The code that uses them is adapted.
- Decoded audio stays what it is: the existing decoding tests pass unchanged, and where the
  new decoder gives different samples for a format, that is found and reported, not hidden.

Out of scope: `ash` (no newer release exists); any other dependency; new audio formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: a requirement that names how dependency versions are kept.

## Impact

- `Cargo.toml`, `Cargo.lock`, `crates/vkxp-audio`, `crates/vkxp-ruby-sys/tests`,
  `xtask/src/oracle`, `deny.toml` if a licence of a new transitive dependency needs listing.
