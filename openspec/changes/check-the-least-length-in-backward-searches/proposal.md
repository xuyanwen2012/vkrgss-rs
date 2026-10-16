# Check the least length of a match in backward searches

## Why

The online check of the review of Ruby 1.9.2 (2026-10-15) found that upstream's fix
33e5b47c ("Fix handling of reg->dmin in Regex matching", which distributions name for
CVE-2024-27282) checks the least length of a match, `reg->dmin`, against the length of the
subject in both `forward_search_range` and `backward_search_range`. The forward check is in
both engines vkrgss builds (CVE-2017-9227). The backward one is in neither: the engine of
Ruby 1.9.2 (`third_party/ruby-1.9.2/regexec.c`) and Oniguruma 5.9.6 of Ruby 1.8.7
(`third_party/oniguruma-5.9.6/regexec.c`) both add `reg->dmin` to `range` without the check,
so a backward search with a pattern longer than the subject can form a pointer past the
subject. Whether a search reads through it was not established.

## What Changes

- `backward_search_range` of both engines returns "not found" when `reg->dmin` exceeds the
  length of the subject, as upstream's fix does. Each is a recorded patch in the
  `PATCHES.md` of its tree.
- A test of each interpreter runs backward searches (`String#rindex` with a pattern, and
  the backward branch of the matcher with each kind of optimization the search can take)
  whose least length exceeds the subject, and gets no match.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the backward search checks the least length.
- `embedded-ruby-1-9-2`: the same.

## Impact

`third_party/ruby-1.9.2` and `third_party/oniguruma-5.9.6` (recorded patches only), the
tests of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` (`tests/backports.rs` where it exists),
`docs/vendored-security.md` (the rows of CVE-2024-27282 and CVE-2017-9227),
`CHANGELOG.md`. No game sees a difference: a match cannot be shorter than its least length,
so the result of every search stays the same.
