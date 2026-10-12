# Security fixes after the vendored releases

vkxp builds two old C trees: Ruby 1.8.7-p374 (June 2013, the last release of its line) and
Oniguruma 5.9.6 (December 2014). Fixes made after those releases reached neither tree. This
document lists the security fixes published since, and says for each what was done about it
here. It was made on 2026-10-12 by the change `fuzz-the-interpreter-and-review-fixes`; a
fix published later is not in it.

Every row has:

- **Identifier**: the CVE, or the upstream issue or commit where there is none.
- **Code**: the file and function concerned.
- **Built**: whether that code is in the vendored tree and compiled into vkxp.
- **Reached by**: whether a game reaches it with a *data* file (a save file or an `.rxdata`
  file, through `Marshal.load`, or a string of such a file), only with a *script*, or not
  at all. A script is code that the game's author wrote, a data file can come from anybody.
- **Outcome**, one of three:
  - **backported** as the recorded patch of that number in `PATCHES.md` of the tree;
  - **does not apply**, with the reason;
  - **left**, with the reason.

The fuzz targets described in [development.md](development.md) found more defects than
this list has; those are fixed by recorded patches too, and the last section names them.

## Sources

- Ruby: the security index of ruby-lang.org (every entry from June 2013 to August 2026);
  the commits of the branch `ruby_1_8_7` after the tag of p374 (three, never released, one
  of them a security fix); the patches of Debian's `ruby1.8` in wheezy (to
  1.8.7.358-7.1+deb7u6, 2018) and squeeze-lts (to 1.8.7.302-2squeeze5, 2015) with the CVE
  list of Debian's security tracker; the patches of `ruby-1.8.7.374-5.el6`, the last build
  of Red Hat Enterprise Linux 6. The errata pages of Red Hat could not be read, so its
  advisories are not cited by number. Red Hat Enterprise Linux 5 has Ruby 1.8.5 and was not
  studied.
- Oniguruma: the history of `github.com/kkos/oniguruma` from the tag `v5.9.6` on (its
  `HISTORY`, the commit messages, the issue numbers) and the CVE entries that name
  Oniguruma. A commit is cited by its short hash. Upstream fixed many defects without a
  CVE, most of them found by fuzzing; they are listed with the issue number or the hash.
  Leaks, time-outs and changes that only avoid undefined pointer arithmetic are not listed.

## Ruby 1.8.7-p374: code that vkxp builds

The built files are the core of the interpreter and `ext/zlib/zlib.c`
(`crates/vkxp-ruby-sys/build.rs` has the list).

| Identifier | Code | Built | Reached by | Outcome |
|---|---|---|---|---|
| CVE-2013-4164 | `util.c`: `ruby_strtod`, `Balloc`, `Bfree` (heap overflow on a very long digit string) | yes | data: a dumped Float in `Marshal.load`; script: `String#to_f`, `Float()`, a literal | **backported**: patch 33 (the `util.c` part of Debian's patch, which is the fix of 1.9.3-p484; the unreleased commit of the branch `ruby_1_8_7` differs only in keeping fewer digits) |
| CVE-2018-8778 | `pack.c`: `pack_unpack` (a count above `LONG_MAX` moves `@`, `X` and `x` out of the string) | yes | script: the format of `String#unpack`; data only where a script builds that format from it | **backported**: patch 34 (the `pack.c` part of Debian's patch) |
| CVE-2018-8780 | `dir.c`: `dir_initialize`, `rb_push_glob`, `dir_globs` (`Dir.open`, `Dir.glob` and others use a path up to its first NUL byte) | yes | script: a path that a script passes | **left**: all of Ruby 1.8 passes a path to the system up to its first NUL byte, `File.open` included (the check came with 1.9), and so does the Ruby 1.8.1 of the original runtime. Scripts pass buffers that a `Win32API` call filled, which end in NUL bytes; refusing them would stop a game that runs on the original. The guest process is confined to the game directory whatever the path is. Not measured on the original |
| CVE-2019-15845 | `dir.c`: `file_s_fnmatch` (`File.fnmatch` stops at a NUL byte in the pattern) | yes | script | **left**: as CVE-2018-8780; no backport for 1.8 exists |
| CVE-2018-16396 | `pack.c`: `pack_pack` (`u`, `m`, `M`, `p`), `pack_unpack` (`b`, `B`, `h`, `H`) do not pass the taint flag on | yes | no effect | **left**: taint is only looked at under `$SAFE` of 1 or more, which vkxp does not use as a boundary; no backport for 1.8 exists |
| CVE-2017-0898 | `sprintf.c`: `rb_str_format`, the formatting of infinity and NaN | file yes, block no | — | **does not apply**: in 1.8.7 that block is inside `#if defined(_WIN32)` |
| CVE-2014-4975 | `pack.c`: `encodes` (a fixed buffer of 4096 bytes) | yes | — | **does not apply**: 1.8.7 sizes the buffer of `encodes` from the count and has no fixed one |
| CVE-2014-3916 | `string.c`: `str_buf_cat` (a too small capacity) | yes | — | **does not apply**: the faulty computation is not in 1.8.7 |
| CVE-2022-28739 | `util.c`: `ruby_strtod`, hexadecimal floats | yes | — | **does not apply**: the `ruby_strtod` of 1.8.7 has no hexadecimal branch |
| CVE-2026-27820 | `ext/zlib/zlib.c`: `zstream_buffer_ungets` | yes | — | **does not apply**: the function is not in 1.8.7; its one-byte `ungetc` grows the buffer first |
| CVE-2017-11465 | `parse.y`: `parser_yyerror` of Ruby 2.4 | yes | — | **does not apply** as published. The `yyerror` of 1.8.7 read and wrote out of bounds in its own way, which the fuzz target `parse` found: patches 30 and 31 |
| CVE-2022-28738, CVE-2024-27282, CVE-2017-6181 | the regular expression engine of later Rubies (Onigmo) | no | — | **does not apply**: `regex.c` of 1.8.7 is not built and Onigmo is not in the tree; the engine vkxp builds is in the next table |

Before p374, and still open here on purpose: the hash-flooding fix of 1.8.7-p357
(CVE-2011-4815) made `String#hash` depend on a random seed. Patch 5 of the tree takes that
out again, because games depend on the order of a `Hash`, which follows the hash values of
the original runtime. Its entry in `PATCHES.md` says so.

## Ruby 1.8.7-p374: code that vkxp does not build

vkxp builds no extension but `Zlib`, no Windows code, and no game loads a file of `lib/`:
the load path is empty, and `require` only takes files inside the game directory. No row
below is built, and none is reached by a game.

| Identifier | Code | Outcome |
|---|---|---|
| CVE-2013-4073 | `lib/openssl/ssl.rb`, host name check (fixed in p374 itself) | **does not apply**: not built |
| CVE-2013-4287, CVE-2013-4363 | RubyGems | **does not apply**: not built |
| CVE-2014-2525 | libyaml, `ext/psych` | **does not apply**: not built |
| CVE-2014-0160, CVE-2014-2734, "Changed default settings of ext/openssl" (2014-10-27) | OpenSSL, `ext/openssl` | **does not apply**: not built |
| CVE-2014-6438 | `lib/uri` | **does not apply**: not built |
| CVE-2014-8080, CVE-2014-8090 | `lib/rexml` (backported by Debian and Red Hat for 1.8.7) | **does not apply**: not built |
| CVE-2015-1855 | `ext/openssl`, host name matching (backported by Debian) | **does not apply**: not built |
| CVE-2015-3900, CVE-2015-4020 | RubyGems | **does not apply**: not built |
| CVE-2015-7551, CVE-2009-5147 | `ext/dl`, fiddle (backported by Debian) | **does not apply**: not built |
| CVE-2015-9096 | `lib/net/smtp` | **does not apply**: not built |
| CVE-2016-2336, CVE-2016-2337, CVE-2016-2338, CVE-2016-2339 | `ext/win32ole`, `ext/tk`, `ext/psych`, `ext/fiddle` | **does not apply**: not built |
| CVE-2016-7798 | `ext/openssl` | **does not apply**: not built |
| CVE-2017-0899 to CVE-2017-0903 | RubyGems | **does not apply**: not built |
| CVE-2017-10784, CVE-2017-17742, CVE-2018-8777 | `lib/webrick` (backported by Debian) | **does not apply**: not built |
| CVE-2017-14033 | `ext/openssl` | **does not apply**: not built |
| CVE-2017-14064 | `ext/json` | **does not apply**: not built |
| CVE-2017-17405 | `lib/net/ftp` (backported by Debian) | **does not apply**: not built |
| CVE-2017-17790 | `lib/resolv.rb` (backported by Debian) | **does not apply**: not built |
| CVE-2018-1000073 to CVE-2018-1000079 | RubyGems | **does not apply**: not built |
| CVE-2018-6914 | `lib/tempfile.rb`, `lib/tmpdir.rb` (backported by Debian) | **does not apply**: not built |
| CVE-2018-8779 | `ext/socket` (backported by Debian) | **does not apply**: not built |
| CVE-2018-16395 | `ext/openssl` | **does not apply**: not built |
| CVE-2019-8320 to CVE-2019-8325 | RubyGems | **does not apply**: not built |
| CVE-2012-6708, CVE-2015-9251 (advisory of 2019-08-28) | rdoc | **does not apply**: not built |
| CVE-2019-16201, CVE-2019-16254, CVE-2020-25613 | `lib/webrick` | **does not apply**: not built |
| CVE-2019-16255 | `lib/shell` | **does not apply**: not built |
| CVE-2020-10663 | `ext/json` | **does not apply**: not built |
| CVE-2020-10933 | `ext/socket` | **does not apply**: not built |
| CVE-2021-28965 | rexml | **does not apply**: not built |
| CVE-2021-28966 | `lib/tmpdir.rb` on Windows | **does not apply**: not built |
| CVE-2021-31799, CVE-2024-27281 | rdoc | **does not apply**: not built |
| CVE-2021-31810 | `lib/net/ftp` | **does not apply**: not built |
| CVE-2021-32066 | `lib/net/imap` | **does not apply**: not built |
| CVE-2021-41817 | `ext/date` | **does not apply**: not built |
| CVE-2021-41816 | `ext/cgi/escape` | **does not apply**: not built |
| CVE-2021-41819, CVE-2021-33621, CVE-2025-27219, CVE-2025-27220 | `lib/cgi` | **does not apply**: not built |
| CVE-2023-28755, CVE-2023-36617, CVE-2025-27221, CVE-2025-61594 | `lib/uri` | **does not apply**: not built |
| CVE-2023-28756 | `lib/time.rb` | **does not apply**: not built |
| CVE-2024-27280 | `ext/stringio` (linked into the test executable of Ruby's suites only) | **does not apply**: not built |
| CVE-2024-35176, CVE-2024-39908, CVE-2024-41123, CVE-2024-41946, CVE-2024-43398, CVE-2024-49761, CVE-2025-58767 | rexml | **does not apply**: not built |
| CVE-2025-0306 | `ext/openssl` | **does not apply**: not built |
| CVE-2025-25186, CVE-2025-43857, CVE-2026-42245, CVE-2026-42246, CVE-2026-42256, CVE-2026-42257, CVE-2026-42258, CVE-2026-47240, CVE-2026-47241, CVE-2026-47242 | net-imap | **does not apply**: not built |
| CVE-2025-24294, CVE-2026-80212, CVE-2026-80213 | resolv | **does not apply**: not built |
| CVE-2026-41316 | erb | **does not apply**: not built |
| CVE-2026-46727 | `ext/socket` | **does not apply**: not built |

## Oniguruma 5.9.6

The interpreter compiles every pattern through `re_compile_pattern` of `reggnu.c` with the
Ruby syntax and one of four encodings (ASCII, UTF-8, Shift_JIS, EUC-JP), and matches with
`re_search` and `re_match`. A pattern is reached by a script (a literal, `Regexp.new`) and
by data (a dumped `Regexp` in `Marshal.load`); a subject is any string. "6.x" below means
code that Oniguruma got after 5.9.6 and that is not in the vendored tree.

| Identifier | Code | Built | Reached by | Outcome |
|---|---|---|---|---|
| #16: 65bdf2a, 151b6b4 | `enc/utf8.c`: `mbc_to_code` reads past a character cut off by the end | yes | data and script | **backported**: patches 2, 3 and 4 |
| #17: 8cdbe15 | `regparse.c`: `PFETCH` steps past the end of the pattern; `regenc.c`: `onigenc_mbc_enc_len_end` | yes | data and script | **backported**: patches 2, 3 and 5 |
| #19: e8e827c | `regparse.c`: `onig_parse_make_tree`, a pattern that ends inside a multibyte character | yes | data and script | **backported**: patch 5. Such a pattern is now the error "too short multibyte code string"; before, the bytes after the pattern were read as its end |
| #18: 44ed4a8 | `enc/utf8.c`: `USE_INVALID_CODE_SCHEME` turned off | yes | — | **does not apply**: a choice of how the bytes `0xfe` and `0xff` are treated, which changes matching; the reads it was made for are covered by patches 2 to 5 |
| #20: dc0a23e | `regparse.c`: `fetch_token` reads after `\k`, `\g`, `\p`, `(?` at the end | yes | data and script | **backported**: patch 5 |
| #21: 1ca27ef | `regparse.c`: `parse_char_class`, a wide character after `--` taken as one byte | yes | data and script | **backported**: patch 5 |
| 68c3955 | `regparse.c`: `parse_enclose`, `(?<` at the end | yes | data and script | **backported**: patch 5 |
| 5186c7c | `regparse.c`: `fetch_token_in_cc`, `[\p{` at the end | yes | data and script | **backported**: patch 5 |
| 562bf48, 162cf91 | `regparse.c`: `fetch_name_with_level`, `\k'-` and `\k'k-0` at the end | yes | data and script | **backported**: patch 5 |
| 8293fd4 | `regparse.c`: `fetch_range_quantifier` | yes | — | **does not apply**: upstream took the line out again (it is commented "Don't do this line" today), and it would drop characters after an invalid `{`. The read it was made for is covered by patch 5 |
| 7fe8c3f | `regcomp.c`: `get_min_match_length`, `get_max_match_length` recurse without end on groups that refer to each other, as in `(\2)(\1)` | yes | data and script | **left**: the fix is a rewrite of about 540 lines that depends on changes of 6.0 which this tree does not have. The pattern exhausts the stack when it is compiled; stack exhaustion is outside this change (`design.md`, "Follow-ups") |
| 9be5a03 | `regparse.c`: `fetch_escaped_value` | yes | — | **does not apply**: a change of types without an access out of bounds in 5.9.6 |
| 9574bc1 | `regcomp.c`: `distance_value` indexes a table with a distance that does not fit a signed number | yes | data and script | **backported**: patch 6 |
| #27: f37dfad | `regparse.c`: `parse_subexp`, use after free on `(|` | yes | — | **does not apply**: the free that it corrects was added after 5.9.6 |
| #29: da316ab | `regcomp.c`: `renumber_by_map` skips the target of an anchor, use after free | yes | data and script | **backported**: patch 6 |
| #40: 6b68ebe, dacfaba; CVE-2019-16163 (#147: 4097828) | `regparse.c`: no limit on the nesting the parser recurses into | yes | data and script | **left**: the fix is a new limit with its own error code and setting, spread over four files. A deeply nested pattern exhausts the stack and ends the process without touching other memory; stack exhaustion is outside this change (`design.md`, "Follow-ups") |
| #43: 5a8a78b; #61: e905508 | `regcomp.c`: captures inside a recursive call | yes | — | **does not apply**: the state it corrects came with 6.0, and upstream took part of it back |
| CVE-2017-9226 (#55: b4bf968, f015fbd) | `regparse.c`: `next_state_val`, `fetch_token`, `fetch_token_in_cc`: an octal escape above 255 written as one byte | yes | data and script | **backported**: patch 5. `\400` to `\777` are now the error "too big number" |
| CVE-2017-9225 (#56: 166a6c3) | `unicode_unfold_key.c` | no | — | **does not apply**: a generated file of 6.x |
| CVE-2017-9224 (#57: 690313a) | `regexec.c`: `match_at`, `OP_EXACT1` reads the subject before it checks its end | yes | data and script | **backported**: patch 7 |
| CVE-2017-9227 (#58: 9690d3a) | `regexec.c`: `forward_search_range`, `reg->dmin` past the end | yes | data and script | **backported**: patch 7 |
| CVE-2017-9229 (#59: b690371) | `regexec.c`: `forward_search_range`, `reg->dmax` before the start | yes | data and script | **backported**: patch 7 |
| CVE-2017-9228 (#60: 3b63d12, ddbf556) | `regparse.c`: `next_state_val`, a range that starts with a class reads an uninitialised value | yes | data and script | **backported**, differently: patch 1 of the tree fixed the same defect earlier and gives the error of the original runtime |
| 00d67cc | `regparse.c`: `add_ctype_to_cc_by_range`, code point overflow | yes | — | **does not apply**: 5.9.6 ends its ranges at `0x7fffffff` and cannot wrap |
| ab17bd4 | `regparse.c`: `str_reduce_to_single_byte_code` | no | — | **does not apply**: 6.x |
| #84: 34c8cdd | `regexec.c`: `match_at`, the back-reference with a nesting level steps past the end | yes | data and script | **backported**: patch 7 |
| #87: 850bd9b, 410f591 | `regerror.c`: `onig_error_code_to_str` reads the name of an error that has none | yes | — | **does not apply**: the error without a name comes from `backref_rel_to_abs` of 6.x; in 5.9.6 every error that takes a name sets it |
| #121: 396a757, ea36d81 | `regparse.c`: missing null checks | no | — | **does not apply**: callouts and other code of 6.x |
| 6fe7b7d | `regexec.c`: `sunday_quick_search` | no | — | **does not apply**: 6.x |
| #139: a8ed18b | `regexec.c`: `match_at`, `OP_MEMORY_END_PUSH_REC` uses a stack pointer after the push that can move the stack | yes | data and script | **backported**: patch 7 |
| CVE-2019-13224 (0f7f61e) | `regext.c`: `onig_new_deluxe` with two encodings, use after free | file yes | no | **does not apply**: the interpreter never calls `onig_new_deluxe` |
| CVE-2019-13225 (c509265) | `regcomp.c`: `compile_bag_node`, if-then-else | no | — | **does not apply**: 6.x |
| 70e9153 | `regenc.c`: `onigenc_mb2_code_to_mbclen`, EUC-KR | no | — | **does not apply**: the encoding is not vendored |
| 6d248fe | `regexec.c`: `match_at`, `OP_CCLASS_MB` reads the subject before it checks its end | yes | data and script | **backported**: patch 7 |
| 1cb2c31 | `regexec.c`: `match_at`, the four back-reference opcodes step past the end after an empty match | yes | data and script | **backported**: patch 7 |
| 7a53a28 | `regparse.c`: `parse_exp`, validity of a string of raw bytes | no | — | **does not apply**: made for the string nodes of 6.x. What 5.9.6 does with bytes that are no valid character is in the last section |
| 8a1c05c | `regexec.c`: the empty check of a look-ahead | no | — | **does not apply**: 6.x |
| CVE-2019-19246 (d3e4029) | `regexec.c`: `str_lower_case_match` reads past the pattern's text | yes | data and script | **backported**: patch 7 |
| #156: 15c4228, adf0fff | `regcomp.c`, `regexec.c`: case-insensitive match reads past the end | part | — | **does not apply**: the part in `regcomp.c` is the byte code of 6.x; the part in `regexec.c` is the function of CVE-2019-19246 |
| CVE-2019-19204 (#162: 6eb4aca) | `regparse.c`: `fetch_range_quantifier` fetches twice for `\{n,m\}` | yes | no | **does not apply**: only with the syntax option `ONIG_SYN_OP_ESC_BRACE_INTERVAL`, which the Ruby syntax does not set |
| CVE-2019-19203 (#163: aa0188e) | `enc/gb18030.c`: `gb18030_mbc_enc_len` | no | — | **does not apply**: the encoding is not vendored |
| CVE-2019-19012 (#164: 0463e21, db64ef3, bfc36d3, 778a43d, b6cb758) | `regexec.c`: `search_in_range`, overflow of `reg->dmax` | no | — | **does not apply** as published: `search_in_range` is 6.x, and the forward search of 5.9.6 has the checks of CVE-2017-9227 and CVE-2017-9229. The backward search of 5.9.6 (`String#rindex` with a pattern) reads one byte after the subject, which for a Ruby string is its terminator: **left**, because the fix moves where a backward search starts, and no backward search was measured on the original runtime (`design.md`, "Follow-ups") |
| 8d37d84 | `regparse.c`: `onig_reduce_nested_quantifier`, overflow of the product of two repeat counts | yes | — | **does not apply**: 5.9.6 does not multiply the counts of nested repeats |
| #167: 603f4db | `regexec.c`: `MATCH_AT_ERROR_RETURN` | no | — | **does not apply**: 6.x |
| e440706 | `regparse.c`: `parse_exp`, use after free | no | — | **does not apply**: the node handling of 6.x |
| #169: 2ce691e | `regcomp.c`: `node_char_len1` | no | — | **does not apply**: 6.x |
| #173: 093d296 | `regcomp.c`: `check_node_in_look_behind` | no | — | **does not apply**: 6.x |
| #175: 56182c7 | `regcomp.c`: `optimize_nodes` | no | — | **does not apply**: 6.x |
| #177: 498d8f7 | `regcomp.c`: `tune_look_behind` | no | — | **does not apply**: 6.x |
| oss-fuzz 22744: 87423b9 | `regparse.c`: `backref_rel_to_abs` | no | — | **does not apply**: 6.x |
| oss-fuzz 22925: 86f9fc7 | `enc/euc_jp.c`: `code_to_mbc` | yes | not found | **left**: the function has the shape the fix corrects, but no pattern was found that passes it a code point outside EUC-JP, and nothing here tests that encoding (`design.md`, "Follow-ups") |
| oss-fuzz 25893: a4aaa67 | `enc/unicode.c`: `onigenc_unicode_get_case_fold_codes_by_str` | yes | — | **does not apply**: the loop it corrects is 6.x. The reads of that function past a cut-off character are fixed by patch 8 |
| CVE-2020-26159 (#207: cbe9f8b, 8155473, 3603d78) | `regcomp.c`: `concat_opt_exact_str` | no | — | **does not apply**: upstream took the fix back as a false report, and the function is 6.x |
| #207: 2493154, 278a172 | `regparse.c`: `make_absent_tree`; `regcomp.c`: `set_empty_repeat_node_trav` | no | — | **does not apply**: 6.x |
| #206: c74dc87, 38b02f9, ed4a253, 47d1654, 2b7b941; f29e9ff, 9d9da5a | `regexec.c`, `regparse.c`, `regcomp.c` | part | — | **does not apply**: functions of 6.x, or pointer arithmetic that is never used out of range |
| 4d3863b | `regparse.c`: the Python syntax | no | — | **does not apply**: not in 5.9.6 |
| 12215ca | `regcomp.c`: `get_tree_tail_literal` | no | — | **does not apply**: 6.x |
| #251: 1877295; #265: 2553552; #267: e6aef5e | `regparse.c`: `prs_branch`, `prs_alts`, `node_new_str_crude_char` | no | — | **does not apply**: 6.x |

## Found by the fuzz targets

The first runs of `cargo xtask fuzz` (`openspec` change
`fuzz-the-interpreter-and-review-fixes`, section "Checked" of its `design.md`) found the
defects of the first rows of the table above again, and these, which no published fix
names. Each is a recorded patch, and its input is a test
(`crates/vkxp-ruby-sys/fuzz/fixed/`).

| Tree | Code | Defect | Patch |
|---|---|---|---|
| Ruby | `parse.y`: `rb_intern` | a name that ends inside a multibyte character is read past its end | 28, 29 |
| Ruby | `parse.y`: `is_special_global_name`, `rb_symname_p` | the same for `$-x` and for the name of a symbol | 30, 31 |
| Ruby | `parse.y`: `yyerror` | the line of a syntax error is copied past its end, and the mark is written past the copy, after a cut-off multibyte character | 30, 31 |
| Ruby | `parse.y`: the rules of `..` and `...`, `assign_in_cond` | a null pointer is read where an operand had a syntax error | 30, 31 |
| Ruby | `re.c`: `rb_reg_expr_str` | the text of a pattern in an error message is copied past its end | 32 |
| Oniguruma | `regparse.c`: `PINC`, `PINC_S` | as `PFETCH` in #17 | 5 |
| Oniguruma | `regcomp.c`: `compile_string_node`, `expand_case_fold_string`; `enc/unicode.c`: `onigenc_unicode_mbc_case_fold`, `onigenc_unicode_get_case_fold_codes_by_str` | a piece of literal text that ends inside a multibyte character is read past its end | 6, 8 |
| Oniguruma | `regexec.c`: `match_at`, `OP_PUSH_OR_JUMP_EXACT1`, `OP_PUSH_IF_PEEK_NEXT` | the subject is read before its end is checked | 7 |

What is known and not fixed: Oniguruma takes a pattern and a subject to be valid in their
encoding. Upstream never changed that; it tells the caller to check. The patches above make
the reads that the fuzz targets reached stay inside the buffers, for UTF-8. A sequence of
bytes that is no character of the encoding can still be read a few bytes past a string
elsewhere, in Shift_JIS and EUC-JP in particular, which the fuzz targets do not select.
The original runtime has the same engine with the same assumption.
