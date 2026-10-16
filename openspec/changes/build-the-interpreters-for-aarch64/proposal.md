# Build the interpreters for aarch64

## Why

P1 of `docs/decisions.md` ("Platforms", ARM Linux) needs both interpreters on aarch64 with a
verified configuration. Today Ruby 1.8.7 is built on aarch64 with the configuration header of
x86_64 (unverified, and it names the platform `x86_64-linux`), and Ruby 1.9.2 builds empty on
aarch64, so games of VX Ace cannot run there at all.

Measured on 2026-10-16 on the owner's Jetson Orin Nano (Ubuntu 24.04.5, gcc 13.3.0): the
releases' own `configure`, run with the flags the x86_64 headers were made with, writes
headers that differ from the checked-in x86_64 ones only in the platform strings
(`RUBY_PLATFORM` and the three `*ARCHLIB` of 1.8.7; `RUBY_PLATFORM` of 1.9.2, besides the two
hand edits of its header). Every `#define` of the hand-written Oniguruma header has the same
value. The headers of that run are in `vkxp/target/p1-config/` of the workspace.

## What Changes

- `config/aarch64-unknown-linux-gnu/` for `vkrgss-ruby-sys` (Ruby and Oniguruma) and for
  `vkrgss-ruby192-sys`: the x86_64 headers with the platform strings of aarch64, each with a
  comment that says where it comes from.
- `vkrgss-ruby-sys` uses its own header on aarch64 instead of borrowing x86_64's.
- `vkrgss-ruby192-sys`, `vkrgss-binding-rgss3`, `vkrgss-guest-main-rgss3` and
  `vkrgss-guest-rgss3` build for aarch64 as for x86_64; other targets stay empty.
- `docs/compatibility.md` says the interpreters build on aarch64 and that P1 is not yet
  accepted.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: a header for aarch64.
- `embedded-ruby-1-9-2`: built on aarch64 too.

## Impact

`crates/vkrgss-ruby-sys`, `crates/vkrgss-ruby192-sys`, the build scripts of
`vkrgss-binding-rgss3`, `vkrgss-guest-main-rgss3` and `vkrgss-guest-rgss3`,
`docs/compatibility.md`, `docs/building.md`, `CHANGELOG.md`. Nothing changes on x86_64. The
CI job `gate-aarch64` stays allowed to fail until P1 is accepted.
