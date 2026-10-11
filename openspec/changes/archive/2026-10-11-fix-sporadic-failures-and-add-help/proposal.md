# Fix two sporadic test failures and add the help option

## Why

Before the gate runs on a shared runner for every push, two tests that fail now and then
must be understood, because either may hide a fault of the runtime:

- `a_guest_that_is_copied_sends_the_frames_it_sends_without`
  (`crates/vkxp-binding/tests/states.rs`) sometimes ends with
  `Script 'Main' line 4: RGSSError occurred. disposed sprite`.
- `the_guest_splits_its_frame_time_when_statistics_are_requested`
  (`crates/vkxp/tests/end_to_end.rs`) sometimes finds no `guest ...` rows in the error
  stream of a run with `--guest process`.

Both failed in two of three runs of `cargo test --workspace --locked` on a busy machine and
in none of eight runs of `cargo test -p vkxp` alone.

Also, `vkxp --help` is an unknown option that prints the usage to the error stream with
status 2, and a few comments and one notice file are out of date after the documentation
moved.

## What Changes

- The cause of each sporadic failure is found and removed, in the runtime if it is there.
- `vkxp --help` and `-h` print the usage to the standard output with status 0.
- Comments that send the reader to the README for text that is now under `docs/` name the
  file; comments that speak of "the development machine" state the general fact.
- `THIRD-PARTY.md` lists the two fonts of the tests of `vkxp-text` with their licences.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the help option.

## Impact

`vkxp-binding`, `vkxp`, possibly `vkxp-host` or `vkxp-proto` where a cause lies; comments in
`vkxp-vk`, `xtask/src/oracle`, `vkxp-core` test data, `vkxp-ruby-sys` tests; `THIRD-PARTY.md`,
`docs/running.md`.
