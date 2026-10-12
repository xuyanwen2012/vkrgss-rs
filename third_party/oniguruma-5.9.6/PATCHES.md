# Local patches to Oniguruma 5.9.6

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

## 1. class-range-start-is-class

- File: `regparse.c`
- Patch: `patches/class-range-start-is-class.patch`
- Patched SHA-256: `6ca285bbffab6a658f0778ab6503c5309b10d0cf2e8b1138365256c2d2dcabe8`
- Reason: found by the first run of the tests on another machine (change
  `fix-findings-of-the-first-ci-runs`), where `/[\d-z]+/` matched `"1-z"`. In
  `parse_char_class` a class such as `\d` leaves the state `CCS_VALUE` with the type
  `CCV_CLASS` and never sets `vs`; a following `-` makes the state `CCS_RANGE`, and the next
  character reaches `next_state_val`, which read the uninitialised `*vs` as the start of the
  range, so the result depended on what the compiler left on the stack. `next_state_val` now
  returns `ONIGERR_MISMATCH_CODE_LENGTH_IN_CLASS_RANGE` in the state `CCS_RANGE` when the
  value before the hyphen is a class, before `*vs` is read. That is the error of the
  original runtime, "mismatch multibyte code length in char-class range". A hyphen followed
  by the end of the class or by `&&` never reaches that state and stays literal.

## 2. mbc-enc-len-end

- File: `regenc.c`
- Patch: `patches/mbc-enc-len-end.patch`
- Patched SHA-256: `a330281069c5001c0d41074c4f9839d337672eb0f9457f4752f81f25bd0a0b65`
- Reason: backport (change `fuzz-the-interpreter-and-review-fixes`, `docs/vendored-security.md`). Adds
  `onigenc_mbc_enc_len_end`, the length of the character at a position but no more than
  what is left before an end, from upstream commit 8cdbe15 (issue #17). Patches 4 to 8 use
  it.

## 3. mbc-enc-len-end-header

- File: `regenc.h`
- Patch: `patches/mbc-enc-len-end-header.patch`
- Patched SHA-256: `831c156f568e73662c033a277c489538cc10b45792e5dbf43d9d1f82a1fd0907`
- Reason: follows patch 2: the declaration and the macro `ONIGENC_MBC_ENC_LEN_END`, from the same upstream commit.

## 4. utf8-mbc-to-code-bounds

- File: `enc/utf8.c`
- Patch: `patches/utf8-mbc-to-code-bounds.patch`
- Patched SHA-256: `50b164c6034debffc78838a33d545959e3a14be53d4a2b56f1393922d9056349`
- Reason: backport of upstream commits 65bdf2a and 151b6b4 (issue #16); found again by
  all three fuzz targets (change `fuzz-the-interpreter-and-review-fixes`). `mbc_to_code` read as many bytes as the lead byte
  announces, past the end of a pattern or of a piece of it that ends inside the character.
  It reads no further than the end it is given.

## 5. parser-bounds

- File: `regparse.c`
- Patch: `patches/parser-bounds.patch`
- Patched SHA-256: `6ca285bbffab6a658f0778ab6503c5309b10d0cf2e8b1138365256c2d2dcabe8`
- Reason: applies after patch 1; backports of the reads and writes out of bounds that
  upstream fixed in the parser after 5.9.6, most of which the fuzz targets found again
  (change `fuzz-the-interpreter-and-review-fixes`; `docs/vendored-security.md` has a row for each). From upstream: `PFETCH` and
  `PFETCH_S` step no further than the end of the pattern (8cdbe15, #17); a pattern that
  ends inside a multibyte character is the error "too short multibyte code string"
  (e8e827c, #19); `fetch_token`, `fetch_token_in_cc`, `parse_enclose` and
  `fetch_name_with_level` test for the end of the pattern before they read after `\k`,
  `\g`, `\p`, `(?`, `(?<`, `[\p{`, `\k'-` and `\k'k-0` (dc0a23e #20, 5186c7c, 68c3955,
  562bf48, 162cf91); in a class, a wide character after `--` is no longer taken for one
  byte (1ca27ef, #21); a value above 255 is not set in the bit set of a class, and an
  octal escape above 255 is the error "too big number" (b4bf968 and f015fbd, #55,
  CVE-2017-9226). Not from upstream: `PINC` and `PINC_S` get the bound of `PFETCH`. Not
  taken: 8293fd4, which upstream withdrew, and 3b63d12 with ddbf556, whose defect patch 1
  fixed with the error of the original runtime. Inputs:
  `crates/vkxp-ruby-sys/fuzz/fixed/regex/{pattern,escape}-incomplete-character.txt` and
  `fuzz/fixed/parse/class-octal-above-255.rb`.

## 6. compiler-bounds

- File: `regcomp.c`
- Patch: `patches/compiler-bounds.patch`
- Patched SHA-256: `9b9e82c937ab51ccc999ff955037d3105a2b2b2256a03a04244368de3043f39f`
- Reason: backports and fixes of the compiler (change `fuzz-the-interpreter-and-review-fixes`; `docs/vendored-security.md`). From
  upstream: `renumber_by_map` also renumbers the target of an anchor, which was a use after
  free for `()(?!(?'a')\1)` (da316ab, #29); `distance_value` keeps the distance unsigned,
  which as a negative number indexed its table far before the start (9574bc1). Not from
  upstream, found by the fuzz targets `marshal` and `parse`: `add_compile_string_length`,
  `compile_string_node` and `expand_case_fold_string` took the length of each character of
  a literal from its lead byte and copied that many bytes; a piece of literal text can end
  inside a multibyte character when its bytes are no valid text, because the parser splits
  it by looking backwards, and the copy then read past the piece. The lengths stop at the
  end of the piece. Inputs:
  `crates/vkxp-ruby-sys/fuzz/fixed/regex/repeat-distance-out-of-range.txt`,
  `fuzz/fixed/marshal/regexp-case-fold-invalid-characters.bin` and
  `fuzz/fixed/parse/regexp-case-fold-invalid-characters.rb`.

## 7. matcher-bounds

- File: `regexec.c`
- Patch: `patches/matcher-bounds.patch`
- Patched SHA-256: `aca6a29018138e842ba18f20db8fa7134819569437820276512fc0a83d645cb8`
- Reason: backports and one fix of the matcher (change `fuzz-the-interpreter-and-review-fixes`; `docs/vendored-security.md` has a
  row for each backport). From upstream: `OP_EXACT1` checks the end of the subject before
  it reads (690313a, #57, CVE-2017-9224); `forward_search_range` fails when `reg->dmin`
  leads past the end and does not step before the start by `reg->dmax` (9690d3a #58,
  CVE-2017-9227; b690371 #59, CVE-2017-9229); the five back-reference opcodes do not look
  for the previous character past the end after an empty match (34c8cdd #84, 1cb2c31);
  `OP_CCLASS_MB` checks the end before it reads (6d248fe); `OP_MEMORY_END_PUSH_REC` takes
  the stack index before the push that can move the stack (a8ed18b, #139);
  `str_lower_case_match` stops at the end of the pattern's text (d3e4029, CVE-2019-19246).
  Not from upstream, found by the fuzz target `regex`: `OP_PUSH_OR_JUMP_EXACT1` and
  `OP_PUSH_IF_PEEK_NEXT` compared the next byte of the subject before they checked that
  there is one, which read past a subject that ends inside a multibyte character. None of
  these changes a result. Input:
  `crates/vkxp-ruby-sys/fuzz/fixed/regex/peek-next-past-the-subject.txt`.

## 8. unicode-case-fold-bounds

- File: `enc/unicode.c`
- Patch: `patches/unicode-case-fold-bounds.patch`
- Patched SHA-256: `cd2156d011088d31489379aeab5353c788824751926fa6102b8c7fb6d96d9d92`
- Reason: found by the fuzz targets `marshal` and `parse` (change `fuzz-the-interpreter-and-review-fixes`).
  `onigenc_unicode_mbc_case_fold` and `onigenc_unicode_get_case_fold_codes_by_str` took the
  length of a character from its lead byte and copied or skipped that many bytes, past the
  end of a text that ends inside the character (see patch 6). The lengths stop at the end
  they are given. Inputs: as for patch 6.
