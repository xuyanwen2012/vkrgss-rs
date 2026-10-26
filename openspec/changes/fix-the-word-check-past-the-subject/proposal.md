# Fix the word check past the subject

## Why

The nightly run 40296906749 (2026-10-26, at `192cd743`) failed in `fuzz (regex)`: an
AddressSanitizer heap-buffer-overflow, a read of 1 byte just after a 58-byte region, in
`mbc_enc_len` (`third_party/oniguruma-5.9.6/enc/utf8.c:65`), called from
`onigenc_mbc_enc_len_end` (`regenc.c:106`) through `mbc_to_code` (`enc/utf8.c:99`) from
`match_at` at `regexec.c:1931`, the word check of `OP_WORD_BOUND`. Patches 2 to 8 of
`third_party/oniguruma-5.9.6/PATCHES.md` bound the length of a character by the end of the
subject, but this path still reads a byte at or past the end before the bound applies. The
same run also put aside three stack overflows in `get_min_match_length` and one timeout,
which the target already counts as limits; they are not part of this change.

The inputs the run saved are at `vkxp/target/fuzz-findings-2026-10-26/regex/` (the overflow
is `crash-a04733116b3f41b160d8af29f7ded68eb83147d1`).

## What Changes

- The read past the subject is reproduced with the saved input, its cause is found and fixed
  as a recorded patch of Oniguruma 5.9.6 (the smallest change; an upstream fix if one is
  known), and the input is kept under `fuzz/fixed/regex/` with the test that replays fixed
  inputs. Every other word check and character read of the matcher that has the same shape
  is checked and fixed in the same patch.
- Ruby 1.9.2's own regular expression engine (`third_party/ruby-1.9.2`, `regexec.c`,
  `enc/utf_8.c`) is checked for the same defect, and fixed the same way if it has it.
- The timeout input is checked: if it is a pattern whose matching is exponential by nature,
  nothing is done; if it is a loop that does not end, it is a finding of this change.
- After the fix, the targets `regex` and `regex192` each run 30 minutes locally without a
  finding outside the listed limits.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the matcher does not read past the subject in its word checks.

## Impact

The fuzz tests of `crates/vkrgss-ruby-sys` and `crates/vkrgss-ruby192-sys`,
`third_party/oniguruma-5.9.6` (and `third_party/ruby-1.9.2` if it has the defect) through
recorded patches, `fuzz/fixed/`, `docs/vendored-security.md` ("Found by the fuzz targets"),
`CHANGELOG.md`.
