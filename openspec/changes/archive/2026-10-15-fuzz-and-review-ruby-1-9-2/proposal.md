# Fuzz Ruby 1.9.2 and review its later security fixes

## Why

The games of RPG Maker VX Ace run Ruby 1.9.2-p0 (August 2010), released in 0.3.0. The tree
of Ruby 1.8.7 has a sanitized run of its suites, three fuzz targets and a review of the
security fixes published after its release (change `fuzz-the-interpreter-and-review-fixes`);
the tree of Ruby 1.9.2 has none of them. The modernization audit of 2026-10-14 found that
it lacked the fix of CVE-2013-4164, a heap overflow that a Float in a save file reaches;
0.3.1 backported that one fix (patch 18). Every other fix published after 1.9.2-p0 is
unchecked, and 1.8.7's first fuzz run found a batch of defects of the kind a fuzzer finds
in minutes, in code 1.9.2 shares in part.

## What Changes

- A sanitized run of Ruby 1.9.2's upstream suite: the interpreter built with clang,
  AddressSanitizer and UndefinedBehaviorSanitizer, with an allow-list as 1.8.7 has. Not a
  gate step.
- Three fuzz targets for Ruby 1.9.2 (Marshal, parser, regular expressions), run by
  `cargo xtask fuzz`. A first run of each; findings fixed as recorded patches with the
  input as a test, or listed.
- A review of the security fixes published for Ruby after 1.9.2-p0, for the code this tree
  builds, as a section of `docs/vendored-security.md`; each backported as a recorded patch
  or given the reason it does not apply or was left.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby-1-9-2`: sanitized run, fuzz targets, review of later fixes.

## Impact

`crates/vkrgss-ruby192-sys` (sanitized test, harnesses, seeds, tests), `third_party/ruby-1.9.2`
(recorded patches only), `xtask` (the targets of `fuzz`), `docs/vendored-security.md`,
`docs/development.md`. A game's behaviour changes only where a defect is fixed; what was
measured on the original runtime of RGSS3 stays.
