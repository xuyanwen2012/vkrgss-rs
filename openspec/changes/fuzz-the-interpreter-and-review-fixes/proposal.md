# Fuzz the interpreter's inputs and review later security fixes

## Why

Three parts of the vendored C code read data that comes with a game and that nobody at
vkxp wrote: `Marshal.load` reads save files and every `.rxdata` file, the parser reads the
scripts, and the regular expression engine compiles the patterns in them. The sanitized run
covers what the upstream suites execute, which is well-formed input. The two defects of
`marshal.c` and the uninitialised range of Oniguruma found on 2026-10-11 were of the kind a
fuzzer finds in minutes.

Ruby 1.8.7-p374 is the last release of its line (June 2013) and Oniguruma 5.9.6 is from
2014. Security fixes made after that reached neither tree, and nobody has checked which of
them concern the code vkxp builds.

## What Changes

- Three fuzz targets (Marshal, parser, regular expressions) built with libFuzzer and the
  sanitizers, and `cargo xtask fuzz` to run one for a given time. Not a gate step.
- A first run of each; what it finds is fixed where the fix is small, with the input as a
  regression test, and listed otherwise.
- A review of the security fixes published for Ruby 1.8 after p374 and for Oniguruma after
  5.9.6: each is backported as a recorded patch, or the review says why it does not apply.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: fuzz targets; the review of later fixes.

## Impact

`crates/vkxp-ruby-sys` (harnesses, seeds, tests), `third_party/ruby-1.8.7` and
`third_party/oniguruma-5.9.6` (recorded patches only), `xtask` (the command),
`docs/development.md`, a new `docs/vendored-security.md`. A game's behaviour changes only
where a defect is fixed; what was measured on the original runtime stays.
