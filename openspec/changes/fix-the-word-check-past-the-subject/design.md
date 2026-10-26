# Design

## Context

- Inputs (read only, copy what is kept into the clone): `../../vkxp/target/fuzz-findings-2026-10-26/regex/`
  holds the five inputs of nightly run 40296906749: `crash-a04733…` (the overflow),
  `stack-*` (three overflows of the stack in `get_min_match_length`, already a listed limit)
  and `timeout-659b40…`.
- The report: READ of size 1 one byte after a 58-byte region; frames `mbc_enc_len`
  (`enc/utf8.c:65`), `onigenc_mbc_enc_len_end` (`regenc.c:106`), `mbc_to_code`
  (`enc/utf8.c:99`), `match_at` (`regexec.c:1931`), `onig_search`, `re_search`,
  `rb_reg_search`, `compile_and_match` (`crates/vkrgss-ruby-sys/fuzz/regex.c:20`).
- Patches 2 to 8 of `third_party/oniguruma-5.9.6/PATCHES.md` and the archived change
  `fuzz-the-interpreter-and-review-fixes` describe how the earlier reads past the subject
  were fixed.

## Decisions

- **Reproduce first** with the saved input through the fuzz executable of `regex`
  (`cargo xtask fuzz regex` builds it). Find which pointer reaches the end: `onigenc_mbc_enc_len_end`
  reads `*p` before it compares with `end`, so a call with `p == end` reads past the
  subject; the word checks of `OP_WORD_BOUND`, `OP_NOT_WORD_BOUND`, `OP_WORD_BEGIN`,
  `OP_WORD_END` and the like are the suspects.
- **Fix.** The smallest correct change, as a recorded patch with its reason, in the style of
  the existing patches; an upstream commit if one is known (there is no network). A guard in
  `onigenc_mbc_enc_len_end` for `p >= end` is acceptable if it is correct for every caller.
- **Ruby 1.9.2.** Check its engine for the same path and run the input (the target `regex192`
  takes the same bytes) through it.
- **The timeout.** Run it alone for longer than the 3 s limit of libFuzzer: if it ends, it is
  backtracking by nature and is recorded here only; if it does not end, find the loop.

## Out of scope

The stack overflows of `get_min_match_length` (already a listed limit); other targets.

## Follow-ups
