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

## Results

- **The cause.** The saved input reproduces the report through `fuzz-regex`. Its pattern is
  `(?<p>\((?:[^(?]|\g<p>)*\b)YYY` and its subject (57 bytes) ends in `+`, `0xf5`, `n`. The
  UTF-8 table of 5.9.6 gives `0xf5` a length of 4, so `OP_CCLASS_NOT` of `[^(?]` stepped
  from two bytes before the end to two bytes past it. There `ON_STR_END` no longer held,
  and `OP_WORD_BOUND` read the byte past the end, as the word character of
  `mbc_to_code`. `onigenc_mbc_enc_len_end` was not the fault: it was called with a
  position past the end because the position was already wrong. A guard there would not
  have helped, because `mbc_to_code` reads `*p` anyway.
- **The other opcodes of that shape.** `match_at` has four opcodes that step over a
  character by the length of its lead byte without checking it against the end:
  `OP_CCLASS`, `OP_CCLASS_NOT`, `OP_WORD` and `OP_NOT_WORD`. Before the fix, each one read
  past the subject under AddressSanitizer with a short input (`[^a]\b` and `[a\x80-\xff]\b`
  against `0xf0`, `\w\b` against `0xf0 0x9f`, `\W\b` and `\W\B` against `0xf7`). The
  others already fail when the character does not fit (`OP_CCLASS_MB`, `OP_CCLASS_NODE`,
  `OP_ANYCHAR` and the star forms, `OP_EXACT*`), or stop at the end (`OP_CCLASS_MB_NOT`,
  and `OP_CCLASS_MIX_NOT` through it). The word checks of `OP_WORD_BOUND`,
  `OP_NOT_WORD_BOUND`, `OP_WORD_BEGIN` and `OP_WORD_END` read `s` only when it is neither
  the start nor the end, and `sprev` is always a character before `s`. So once `s` never
  passes the end, they stay inside the subject. Ruby's syntax does not have `\<` and `\>`,
  so `OP_WORD_BEGIN` and `OP_WORD_END` cannot be reached anyway. The steps of the search
  loops of `onig_search` that may pass the end are compared with `range` before they are
  used.
- **The fix.** Patch 10 of `third_party/oniguruma-5.9.6/PATCHES.md`
  (`step-to-the-end-of-the-subject`): the four opcodes stop at the end, as
  `OP_CCLASS_MB_NOT` of 5.9.6 already does with a character cut off by the end. No upstream
  commit is known (there is no network). Later upstream releases leave the validity of the
  subject to the caller (`ONIG_OPTION_CHECK_VALIDITY_OF_STRING`). Patches 7 and 9 now
  record the new hash of `regexec.c`. The saved input, the short inputs above and a run of
  300 s of `regex` with a scratch trap in `onigenc_mbc_enc_len_end` for `p >= end` (not
  committed) reported nothing. That trap also covers a read at the end itself, which
  AddressSanitizer cannot see because a Ruby string has a terminator there. The input is
  `crates/vkrgss-ruby-sys/fuzz/fixed/regex/word-bound-past-the-subject.txt`. The test of
  fixed inputs also checks that `"x\xf7" =~ /[^x]/` and `"\xf0\x9f" =~ /\w+/` end their
  match at the end of the subject. Before the fix these matches ended at 5 and 4, past
  subjects of 2 bytes.
- **Ruby 1.9.2.** Its `regexec.c` has the same four opcodes without the check.
  `enclen(enc, p, end)` there is `onigenc_mbclen_approximate`, which for a character cut
  off by the end gives the bytes left plus the bytes still needed, so the same step would
  pass the end. That path is not reached, though: `rb_reg_search` is the only caller of
  `onig_search`, and `rb_reg_prepare_enc` raises `ArgumentError` ("invalid byte sequence")
  for a subject that is no valid text before the engine runs. In valid text no character
  runs past the end. The saved input and the short inputs above report nothing through
  `fuzz-regex192`. The tree is therefore not patched; `docs/vendored-security.md` has a row
  saying so.
- **The runs of 30 minutes**, after the fix, on 2026-10-26, both at once on this machine:
  `cargo xtask fuzz regex --seconds 1800` ran 2,444,575 inputs in 1808 s without a finding.
  79 inputs ran into the time limit of 3 s, 2 exhausted the stack by nesting, and no
  process ran into the memory limit. `cargo xtask fuzz regex192 --seconds 1800` ran
  10,574,883 inputs in 1802 s without a finding. 145 inputs ran into the time limit and
  none exhausted the stack or the memory. Both corpora started from those of the earlier,
  shorter runs of this change.

## Follow-ups

- **The timeout input** (`timeout-659b406ef2784ac0d0af44bd74e0bdce9b58754e`) is
  backtracking by nature, not a loop. Its pattern is `.?++?+**b2?x?+,3A\xffA`, nested
  repeats of `.`, and its subject is 62 bytes. Run alone through `fuzz-regex` with
  `-timeout=0`, it had not ended after 10 minutes. Its prefixes, with the same pattern, end
  in 0.02 s up to 19 bytes, then take 1.4, 2.7, 5.2, 10.4, 20.7 and 42.8 s for 20 to 25
  bytes. The time doubles with each byte, so the whole subject would need about
  1.4 s × 2^42. Nothing is changed; it is one of the inputs that the target already sets
  aside at the time limit.
