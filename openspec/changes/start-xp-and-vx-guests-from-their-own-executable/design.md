# Design

## Context

`crates/vkrgss/src/process.rs` (`RGSS3_GUEST`, `guest_executable`, `guest_executable_of`)
chooses the executable of the guest process: the running program for RGSS1 and RGSS2,
`vkrgss-guest-rgss3` beside it for RGSS3. `crates/vkrgss-guest-rgss3` is a binary of a few
lines over `vkrgss-guest-main-rgss3::guest_process`. `vkrgss-guest-main` is the same source
over Ruby 1.8.7.

## Decisions

- **Name.** `vkrgss-guest-rgss12`: it serves RGSS1 and RGSS2, which share Ruby 1.8.7, and
  sits beside `vkrgss-guest-rgss3`. (A naming choice made while the owner was away; it is
  one constant and one package name to change if the owner prefers another.)
- **Package.** A copy of the shape of `crates/vkrgss-guest-rgss3` (manifest, `main.rs`, the
  `cfg` for a target without the interpreter if `vkrgss-guest-main` has one, the message of
  `not_a_guest` naming XP and VX), depending on `vkrgss-guest-main`. Where `xtask` or tests
  check `vkrgss-guest-rgss3` (dependency lists, version equality, nothing of the host
  linked), check `vkrgss-guest-rgss12` the same way.
- **Program.** `guest_executable_of` returns `vkrgss-guest-rgss12` beside the program for
  RGSS1 and RGSS2. The hidden guest role of `vkrgss` itself is removed if nothing else uses
  it (a guest on a thread does not go through it); if a test or the copier of save states
  needs it, say so here and keep only what is needed.
- **Tests of the program.** The requirement "Tests of the program reach the guest
  executable beside it" already makes the tests of `vkrgss` find `vkrgss-guest-rgss3`; the
  same mechanism finds `vkrgss-guest-rgss12`.
- **Package.** `cargo xtask dist` builds and checks the three executables (static libcrypt
  in each).
- **Save states, sandbox, versions.** Unchanged: they live in `vkrgss-guest` and
  `vkrgss-guest-main`, which the new executable runs exactly as `vkrgss` ran them in the
  guest role. The version statement is the package version, which equals the workspace's.

## Out of scope

macOS itself; moving the socket code out of `vkrgss-proto` (the audit's item 20 names it,
but it is not needed for the executable split).

## Follow-ups

None yet.
