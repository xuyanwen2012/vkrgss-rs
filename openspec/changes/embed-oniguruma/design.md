# Design

## Context

- The original DLL contains Oniguruma's error messages ("premature end of char-class", "end
  pattern with unmatched parenthesis", ...). Measured with a probe script in the original
  (RGSS103J, Ruby 1.8.1) and in vkxp: 65 lines, 17 differ. The original's output and the probe
  are in this clone at `local/measurements/regex.original.txt` and `regex-probe.rb` (ignored
  by git); the output is committed by this change as test data, being measured behaviour.
- `third_party/ruby-1.8.7` compiles `regex.c` (the interpreter's GNU-style engine) and `re.c`,
  which calls it through `regex.h`: `re_compile_pattern`, `re_search`, `re_match`,
  `re_adjust_startpos`, `re_mbcinit`, `re_set_casetable`, `re_free_pattern`,
  `re_free_registers`, `re_copy_registers`, and fields of `struct re_pattern_buffer` and
  `struct re_registers`. `string.c` and others use `ismbchar`, `mbclen` and the
  `re_mbctab` table from the same header.
- Oniguruma 5.9.6 (BSD licence) is the last release line that ships a GNU-compatible layer for
  exactly this use: `oniggnu.h` and `reggnu.c` provide the `re_*` functions and the pattern and
  register structures. Its release tarball is in this clone at `local/onig-5.9.6.tar.gz`
  (SHA-256 `d5642010336a6f68b7f2e34b1f1cb14be333e4d95c2ac02b38c162caf44e47a7`). Earlier
  Oniguruma releases carried a ready `re.c` patch for Ruby 1.8; 5.9.6 does not, so `re.c` is
  adapted by hand.
- Vendored interpreter files are pinned by `MANIFEST.sha256`; patches are recorded in
  `PATCHES.md` with patch files, and a test checks both.

## Decisions

- Vendor the needed part of the release under `third_party/oniguruma-5.9.6/` (the engine's
  sources and headers, the encodings for ASCII, UTF-8, Shift_JIS and EUC-JP, `COPYING`), with a
  manifest and a test like the interpreter's, and compile it in the build script of
  `vkxp-ruby-sys` with a `config.h` written for the purpose.
- `re.c` is patched to use `oniggnu.h` in place of `regex.h`, with the Ruby syntax
  (`ONIG_SYNTAX_RUBY`) and the encoding chosen from `$KCODE` and the expression's flag as
  Ruby 1.8 with Oniguruma did; the multi-byte helpers that other files take from `regex.h`
  are provided from Oniguruma's encoding functions in a small compatibility header, so that
  `string.c` and the rest are changed as little as possible. `regex.c` leaves the file list.
- Error messages come from Oniguruma, which is what the measured output shows.
- Where 5.9.6 differs from the measured output of the original's older Oniguruma, the
  difference is listed under Follow-ups rather than patched in the engine, unless it is a
  matter of the options this change sets.

## Risks / Trade-offs

- [Other code that relied on the former engine's quirks] → The upstream suites and the
  binding's tests run in the gate.
- [A second vendored C library] → It is small, pinned and built the same way as the
  interpreter.

## Follow-ups

- Behaviour that differs between Oniguruma versions and that no probe has shown yet.
- Differences from the measured output that remain; the line numbers are those of
  `crates/vkxp-ruby-sys/tests/data/regex.original.txt`, and the test adjusts exactly these
  before it compares:
  - Line 40, `/[\d-z]+/`: an error in both, but the original says "mismatch multibyte code
    length in char-class range" and Oniguruma 5.9.6 says "empty range in char class". The
    message is chosen inside the engine's parser, not by an option.
  - Line 17, `/../n` on a UTF-8 subject: the capture is the same two bytes. `String#inspect`
    of the original's Ruby 1.8.1 prints an incomplete multi-byte character raw, together
    with the byte after the string; 1.8.7 prints escapes. This is the interpreter's
    `inspect`, not the engine; whether to follow 1.8.1 here is open.
  - Line 43, `/#{1+1}/`: the result is equal. `String#inspect` of 1.8.1 does not escape `#{`
    and 1.8.7 does, so the test takes the pattern's text from the measured line.
- Four measured lines (11, 12, 19, 20) are matched by options: the syntax is
  `ONIG_SYNTAX_RUBY` without `ONIG_SYN_OP2_ESC_H_XDIGIT`,
  `ONIG_SYN_OP2_ESC_P_BRACE_CHAR_PROPERTY`, `ONIG_SYN_OP2_ESC_P_BRACE_CIRCUMFLEX_NOT`,
  `ONIG_SYN_ALLOW_INTERVAL_LOW_ABBREV` and `ONIG_SYN_FIXED_INTERVAL_IS_GREEDY_ONLY`. Other
  options of the 5.9.6 Ruby syntax that the original's engine may lack have not been probed.
- Backward search (`rindex`, `start_with?`) is a loop of anchored matches in `re.c`, because
  the backward search of 5.9.6 rejects a match that reaches beyond the character at the start
  position (a compile-time setting inside `regexec.c`) and the upstream suite needs
  `start_with?` to work. How the original behaves for `rindex` with a pattern has not been
  measured.
- Oniguruma's warnings (a redundant nested repeat, an unescaped `-` or `]` in a class) are not
  connected to `rb_warn`; what the original prints for them has not been measured.
- `README.md` and `docs/decisions.md` still describe the stock engine and the LGPL of
  `regex.c` as built; they are outside this change's files. `regex.c` and `regex.h` stay in
  the vendored tree unbuilt; removing them is a separate question.
