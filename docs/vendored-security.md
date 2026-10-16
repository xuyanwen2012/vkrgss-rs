# Security fixes after the vendored releases

vkrgss builds three old C trees: Ruby 1.8.7-p374 (June 2013, the last release of its line),
Oniguruma 5.9.6 (December 2014), and Ruby 1.9.2-p0 (August 2010) with its own regular
expression engine. Fixes made after those releases reached none of the trees. This document
lists the security fixes published since, and says for each what was done about it here.
The sections on Ruby 1.8.7 and Oniguruma 5.9.6 were made on 2026-10-12 by the change
`fuzz-the-interpreter-and-review-fixes`, the section on Ruby 1.9.2 on 2026-10-15 by the
change `fuzz-and-review-ruby-1-9-2`; a fix published later is not in them.

Every row has:

- **Identifier**: the CVE, or the upstream issue or commit where there is none.
- **Code**: the file and function concerned.
- **Built**: whether that code is in the vendored tree and compiled into vkrgss.
- **Reached by**: whether a game reaches it with a *data* file (a save file or an `.rxdata`,
  `.rvdata` or `.rvdata2` file, through `Marshal.load`, or a string of such a file), only
  with a *script*, or not at all. A script is code that the game's author wrote, a data file can come from anybody.
- **Outcome**, one of three:
  - **backported** as the recorded patch of that number in `PATCHES.md` of the tree;
  - **does not apply**, with the reason;
  - **left**, with the reason.

The fuzz targets described in [development.md](development.md) found more defects than
this list has; those are fixed by recorded patches too, and a section of each tree of Ruby
names them.

## Sources

The sources of the sections on Ruby 1.8.7 and Oniguruma 5.9.6; those of Ruby 1.9.2 are in
its own section.

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

## Ruby 1.8.7-p374: code that vkrgss builds

The built files are the core of the interpreter and `ext/zlib/zlib.c`
(`crates/vkrgss-ruby-sys/build.rs` has the list).

| Identifier | Code | Built | Reached by | Outcome |
|---|---|---|---|---|
| CVE-2013-4164 | `util.c`: `ruby_strtod`, `Balloc`, `Bfree` (heap overflow on a very long digit string) | yes | data: a dumped Float in `Marshal.load`; script: `String#to_f`, `Float()`, a literal | **backported**: patch 33 (the `util.c` part of Debian's patch, which is the fix of 1.9.3-p484; the unreleased commit of the branch `ruby_1_8_7` differs only in keeping fewer digits) |
| CVE-2018-8778 | `pack.c`: `pack_unpack` (a count above `LONG_MAX` moves `@`, `X` and `x` out of the string) | yes | script: the format of `String#unpack`; data only where a script builds that format from it | **backported**: patch 34 (the `pack.c` part of Debian's patch) |
| CVE-2018-8780 | `dir.c`: `dir_initialize`, `rb_push_glob`, `dir_globs` (`Dir.open`, `Dir.glob` and others use a path up to its first NUL byte) | yes | script: a path that a script passes | **left**: all of Ruby 1.8 passes a path to the system up to its first NUL byte, `File.open` included (the check came with 1.9), and so does the Ruby 1.8.1 of the original runtime. Scripts pass buffers that a `Win32API` call filled, which end in NUL bytes; refusing them would stop a game that runs on the original. The guest process is confined to the game directory whatever the path is. Not measured on the original |
| CVE-2019-15845 | `dir.c`: `file_s_fnmatch` (`File.fnmatch` stops at a NUL byte in the pattern) | yes | script | **left**: as CVE-2018-8780; no backport for 1.8 exists |
| CVE-2018-16396 | `pack.c`: `pack_pack` (`u`, `m`, `M`, `p`), `pack_unpack` (`b`, `B`, `h`, `H`) do not pass the taint flag on | yes | no effect | **left**: taint is only looked at under `$SAFE` of 1 or more, which vkrgss does not use as a boundary; no backport for 1.8 exists |
| CVE-2017-0898 | `sprintf.c`: `rb_str_format`, the formatting of infinity and NaN | file yes, block no | — | **does not apply**: in 1.8.7 that block is inside `#if defined(_WIN32)` |
| CVE-2014-4975 | `pack.c`: `encodes` (a fixed buffer of 4096 bytes) | yes | — | **does not apply**: 1.8.7 sizes the buffer of `encodes` from the count and has no fixed one |
| CVE-2014-3916 | `string.c`: `str_buf_cat` (a too small capacity) | yes | — | **does not apply**: the faulty computation is not in 1.8.7 |
| CVE-2022-28739 | `util.c`: `ruby_strtod`, hexadecimal floats | yes | — | **does not apply**: the `ruby_strtod` of 1.8.7 has no hexadecimal branch |
| CVE-2026-27820 | `ext/zlib/zlib.c`: `zstream_buffer_ungets` | yes | — | **does not apply**: the function is not in 1.8.7; its one-byte `ungetc` grows the buffer first |
| CVE-2017-11465 | `parse.y`: `parser_yyerror` of Ruby 2.4 | yes | — | **does not apply** as published. The `yyerror` of 1.8.7 read and wrote out of bounds in its own way, which the fuzz target `parse` found: patches 30 and 31 |
| CVE-2022-28738, CVE-2024-27282, CVE-2017-6181 | the regular expression engine of later Rubies (Onigmo) | no | — | **does not apply**: `regex.c` of 1.8.7 is not built and Onigmo is not in the tree; the engine vkrgss builds is in the next table, where the backward check of 33e5b47c is patch 9 (row of CVE-2017-9227) |

Before p374, and still open here on purpose: the hash-flooding fix of 1.8.7-p357
(CVE-2011-4815) made `String#hash` depend on a random seed. Patch 5 of the tree takes that
out again, because games depend on the order of a `Hash`, which follows the hash values of
the original runtime. Its entry in `PATCHES.md` says so.

## Ruby 1.8.7-p374: code that vkrgss does not build

vkrgss builds no extension but `Zlib`, no Windows code, and no game loads a file of `lib/`:
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
| CVE-2017-9227 (#58: 9690d3a) | `regexec.c`: `forward_search_range`, `reg->dmin` past the end | yes | data and script | **backported**: patch 7. The same check in `backward_search_range`, from ruby/ruby 33e5b47c (named for CVE-2024-27282), is patch 9: a least distance that does not fit 32 bits is the infinite one, which no threshold turns away, and a backward search (`String#rindex`) formed a position about 4 GiB past the subject |
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

The first runs of the targets of Ruby 1.8.7, `marshal`, `parse` and `regex` (`openspec` change
`fuzz-the-interpreter-and-review-fixes`, section "Checked" of its `design.md`) found the
defects of the first rows of the table above again, and these, which no published fix
names. Each is a recorded patch, and its input is a test
(`crates/vkrgss-ruby-sys/fuzz/fixed/`).

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

## Ruby 1.9.2-p0

The tree of Ruby 1.9.2-p0 (August 2010), which games of RGSS3 run, was reviewed on
2026-10-15 by the change `fuzz-and-review-ruby-1-9-2`. Its rows are in the tables below, in
the form of those above. "Data" here is a save file or an `.rvdata2` file, through
`Marshal.load`; the original runtime of RGSS3 is this very release, so where a fix would
change what a game sees, the original's behaviour is known without a measurement.

### Sources

The network was not reachable during the review, so the sources the change names were not
consulted: not the security index of ruby-lang.org, not the change logs of the patch
releases p136 to p330, not the patches of Debian's `ruby1.9.1`, not the repositories of
Oniguruma and Onigmo. The list was made from what the trees show (the tree of 1.9.2 itself,
the tree of 1.8.7 with its patches, and the rows above, which were made with those sources
in 2026-10) and from the reviewer's knowledge of the advisories, each named by its CVE. A
row whose upstream fix could not be compared says so. Every fix of the rows that are
backported was written against this tree and shown with a test where a few lines can show
the defect; the defects of the rows that are left were read in this tree.

The section was checked against those sources afterwards, on 2026-10-15, with the network:
the security index and news of ruby-lang.org, the release notes of p136 to p330, NVD, the
fix commits of ruby/ruby, Oniguruma and Onigmo, and the patch series of Debian's
`ruby1.9.1` for squeeze-lts (1.9.2.0-2+deb6u7) and wheezy (1.9.3.194-8.1+deb7u8). Every fix
of the core's C code that Debian shipped maps to a row; the backports of patches 18, 26,
27, 29 and 33 to 36 were compared with the upstream fixes and are equivalent (patch 31 is a
point fix where upstream rewrote the branch, as its row says). The check corrected the row
of CVE-2024-27282 and added the rows of CVE-2019-16161, CVE-2019-16162, CVE-2010-2489 and
CVE-2009-4124. Not compared in that check: the upstream diffs of patches 30 (CVE-2018-8778)
and 32 (CVE-2014-3916), and the recorded checksums of `PATCHES.md`.

### Code that vkrgss builds

The built files are the core of the interpreter with its own regular expression engine (a
fork of Oniguruma 5.9 that Ruby carries), the encodings and transcoders that the original
runtime has, and `ext/zlib/zlib.c` (`crates/vkrgss-ruby192-sys/build.rs` has the list).

| Identifier | Code | Built | Reached by | Outcome |
|---|---|---|---|---|
| CVE-2013-4164 | `util.c`: `ruby_strtod`, `Balloc`, `Bfree` (heap overflow on a very long digit string) | yes | data: a dumped Float in `Marshal.load`; script: `String#to_f`, `Float()`, a literal | **backported**: patch 18 (since 0.3.1) |
| CVE-2014-4975 | `pack.c`: `encodes` (the line feed after a last group that filled the fixed buffer of 4096 bytes is written one byte past it, on the stack) | yes | script: `Array#pack` with `m` and a count, as `['a' * 3071].pack('m3072')`; data only where a script packs it so | **backported**: patch 29 (the buffer of upstream's fix, one byte longer than what the loop fills) |
| CVE-2018-8778 | `pack.c`: `pack_unpack` (a count above `LONG_MAX` is negative, and `@`, `X` and `x` move out of the string) | yes | script: the format of `String#unpack`; data only where a script builds that format from it | **backported**: patch 30 (the check of upstream's fix and of patch 34 of the tree of 1.8.7) |
| CVE-2017-0898 | `sprintf.c`: `rb_str_format`, the formatting of infinity and NaN (with the largest width the size of the buffer wraps, the buffer does not grow, and the text is written as far as the width past it) | yes | script: a format with a width of 2147483647, given or through `*`, and a non-finite Float | **backported**, differently: patch 31 refuses that width with the error of a width too big in the format; upstream rewrote the block. The finite branch is in "Follow-ups" of the change |
| CVE-2014-3916 | `string.c`: `str_buf_cat` (a too small capacity when the string is near `LONG_MAX / 2` bytes) | yes | not in practice: on x86_64 the branch needs a string of 2^62 bytes | **backported**: patch 32 (the rounding of upstream's fix) |
| CVE-2026-27820 | `ext/zlib/zlib.c`: `zstream_buffer_ungets` (bytes put back are written past the buffer when less than their length is free) | yes | script: `Zlib::GzipReader#ungetc` with a string | **backported**: patch 33 (the buffer grows when less than the length is free; upstream's fix was not compared) |
| CVE-2022-28739 | `util.c`: `ruby_strtod`, hexadecimal floats | yes | — | **does not apply**: every loop of the hexadecimal branch of 1.9.2 tests for the terminator before it looks a character up; the read past the string came with the later rewrite of that branch |
| CVE-2018-8780 | `dir.c`: `dir_initialize` (`Dir.open`, `Dir.new`, `Dir.entries` and `Dir.foreach` use a path up to its first NUL byte, through `GlobPathValue`); `Dir.glob` takes NUL bytes as separators by design | yes | script: a path that a script passes | **left**: the original runtime of RGSS3 is this release and opens such a path the same way. Scripts pass buffers that a `Win32API` call filled, which end in NUL bytes; refusing them would stop a game that runs on the original. The guest process is confined to the game directory whatever the path is. `File.open` and the other functions of `file.c` refuse a NUL byte in 1.9.2 (`rb_get_path_check`) |
| CVE-2019-15845 | `dir.c`: `file_s_fnmatch` (`File.fnmatch` stops at a NUL byte in the pattern) | yes | script | **left**: as CVE-2018-8780 |
| CVE-2012-4522 | `file.c`: `rb_get_path_check` (a NUL byte in a path of `File.open` and others) | yes | — | **does not apply**: 1.9.2 refuses the NUL byte (`StringValueCStr`); the defect came with a change of 1.9.3 |
| CVE-2011-4815, CVE-2012-5371 | `random.c`, `st.c`, `string.c`: the seeded hash of strings and other values (hash flooding: keys chosen to collide make a `Hash` slow) | yes | data: the keys of a dumped `Hash` | **left**: patch 6 fixes the seed on purpose, so that a run is repeatable; its entry in `PATCHES.md` says so. The fix of CVE-2012-5371 is a new hash function (SipHash), whose values a fixed seed would make as predictable. A file built to collide makes loading slow and touches no memory it should not; a save file is the player's own |
| CVE-2011-2686 | `random.c`, `process.c`: the seed of `rand` after `fork` | yes | — | **does not apply**: the defect is a regression of 1.8.7; 1.9.2 draws a new seed in a child of its own `fork` (`rb_thread_atfork` calls `rb_reset_random_seed`). The copy of the process that vkrgss makes for a state keeps the sequence of `rand` on purpose, because the state continues the same run |
| CVE-2011-1005 | `error.c`: `exc_to_s` and `$SAFE` | yes | — | **does not apply**: the advisory names Ruby 1.8 alone |
| CVE-2012-4464, CVE-2012-4466 | `error.c`: `exc_to_s`, `name_err_to_s` (the taint flag of the message under `$SAFE`) | yes | no effect | **left**: taint is only looked at under `$SAFE` of 1 or more, which vkrgss does not use as a boundary |
| CVE-2018-16396 | `pack.c`: `pack_pack`, `pack_unpack` do not pass the taint flag on | yes | no effect | **left**: as CVE-2012-4464 |
| CVE-2017-11465 | `parse.y`: `parser_yyerror` of Ruby 2.4 | yes | — | **does not apply**, as far as could be established without comparing the published fix: the lexer of 1.9.2 refuses an incomplete multibyte character before it steps over it, so the position of an error stays inside its line, which the copy of `parser_yyerror` is bounded by (the defect that patches 30 and 31 of the tree of 1.8.7 fix in its `yyerror`); the target `parse192` found no defect there |
| CVE-2022-28738 | `re.c` of Ruby 3.0 and 3.1, double free in the compilation of a pattern | no | — | **does not apply**: the code came with 3.0 |
| CVE-2024-27282 | the matcher of Onigmo in Ruby 3.0 to 3.3 (an address read with a pattern search) | the predecessor of that code | data and script | **backported** in its main part: the fix of ruby/ruby (989a2355, "Fix Use-After-Free issue for Regexp") moves `GET_STACK_INDEX` before `STACK_PUSH_MEM_END` in `OP_MEMORY_END_PUSH_REC`, which is #139 (a8ed18b) of the engine table, patch 35. Distributions also name 33e5b47c, which checks `reg->dmin` against the length of the subject in both `forward_search_range` and `backward_search_range`: the forward check is CVE-2017-9227 (patch 35); the backward one is **backported**: patch 37. A backward search (`String#rindex`) reached it with nested repeats whose least distance is the infinite one, for which no threshold is set, and the addition wrapped around the address space, which the sanitized build reported; `tests/backports.rs` shows it |
| CVE-2017-6181 | `regparse.c`: `parse_char_class`, recursion without a limit on nested classes | yes | data and script | **left**: a deeply nested pattern exhausts the stack and ends the process without touching other memory, as #40 and CVE-2019-16163 below |

The two candidates the change named as "the `Marshal` and `String#to_f` fixes of 1.9.3"
came down to CVE-2013-4164 above for `String#to_f`; no published security fix of `marshal.c`
after 1.9.2 is known to the reviewer, and `Marshal.load` was fuzzed instead (the target
`marshal192`).

### Code that vkrgss does not build

vkrgss builds no extension of this tree but `Zlib`, no Windows code, and no game loads a
file of `lib/`: the load path is the game directory alone. The extensions `stringio`, `fcntl`, `etc`, `continuation` and `fiber` and the files
of `lib/` that are vendored are linked into, or loaded by, the test executable of Ruby's
suites only. No row below is built into vkrgss, and none is reached by a game.

| Identifier | Code | Outcome |
|---|---|---|
| CVE-2010-0541, CVE-2017-10784, CVE-2017-17742, CVE-2018-8777, CVE-2019-16201, CVE-2019-16254, CVE-2020-25613 | `lib/webrick` | **does not apply**: not vendored |
| CVE-2010-2489, CVE-2009-4124 | `io.c`: `ARGF.inplace_mode` on Windows; `string.c` of 1.9.1 | **does not apply**: fixed before 1.9.2-p0, and the first is Windows code |
| CVE-2011-1004 | `lib/fileutils.rb`, `remove_entry_secure` | **does not apply**: vendored for the test runs only, never loaded by a game |
| CVE-2011-0188 | `ext/bigdecimal` | **does not apply**: not vendored |
| CVE-2011-2705 | `lib/securerandom.rb` | **does not apply**: not vendored |
| CVE-2013-4073, CVE-2014-0160, CVE-2014-2734, CVE-2015-1855, CVE-2016-7798, CVE-2017-14033, CVE-2018-16395, CVE-2025-0306, "Changed default settings of ext/openssl" (2014-10-27) | OpenSSL, `ext/openssl` | **does not apply**: not vendored |
| CVE-2012-2125, CVE-2012-2126, CVE-2013-4287, CVE-2013-4363, CVE-2015-3900, CVE-2015-4020, CVE-2017-0899 to CVE-2017-0903, CVE-2018-1000073 to CVE-2018-1000079, CVE-2019-8320 to CVE-2019-8325 | RubyGems | **does not apply**: not vendored |
| CVE-2013-0256, CVE-2012-6708, CVE-2015-9251, CVE-2021-31799, CVE-2024-27281 | rdoc | **does not apply**: not vendored |
| CVE-2013-0269, CVE-2017-14064, CVE-2020-10663 | `ext/json` | **does not apply**: not vendored |
| CVE-2013-1821, CVE-2014-8080, CVE-2014-8090, CVE-2021-28965, CVE-2024-35176, CVE-2024-39908, CVE-2024-41123, CVE-2024-41946, CVE-2024-43398, CVE-2024-49761, CVE-2025-58767 | `lib/rexml` | **does not apply**: not vendored |
| CVE-2013-2065, CVE-2015-7551, CVE-2009-5147, CVE-2016-2339 | `ext/dl`, fiddle (vkrgss's `DL` of RGSS3 is its own code) | **does not apply**: not vendored |
| CVE-2014-2525, CVE-2016-2338 | libyaml, `ext/psych` | **does not apply**: not vendored |
| CVE-2014-6438, CVE-2023-28755, CVE-2023-36617, CVE-2025-27221, CVE-2025-61594 | `lib/uri` | **does not apply**: not vendored |
| CVE-2015-9096 | `lib/net/smtp` | **does not apply**: not vendored |
| CVE-2016-2336, CVE-2016-2337 | `ext/win32ole`, `ext/tk` | **does not apply**: not vendored |
| CVE-2017-17405, CVE-2021-31810 | `lib/net/ftp` | **does not apply**: not vendored |
| CVE-2017-17790, CVE-2025-24294, CVE-2026-80212, CVE-2026-80213 | `lib/resolv.rb` | **does not apply**: not vendored |
| CVE-2018-6914, CVE-2021-28966 | `lib/tempfile.rb`, `lib/tmpdir.rb` (vendored for the test runs) | **does not apply**: never loaded by a game |
| CVE-2018-8779, CVE-2020-10933, CVE-2026-46727 | `ext/socket` | **does not apply**: not vendored |
| CVE-2019-16255 | `lib/shell` | **does not apply**: not vendored |
| CVE-2021-32066, CVE-2025-25186, CVE-2025-43857, CVE-2026-42245, CVE-2026-42246, CVE-2026-42256, CVE-2026-42257, CVE-2026-42258, CVE-2026-47240, CVE-2026-47241, CVE-2026-47242 | net-imap | **does not apply**: not vendored |
| CVE-2021-41817 | `ext/date` | **does not apply**: not vendored |
| CVE-2021-41816, CVE-2021-41819, CVE-2021-33621, CVE-2025-27219, CVE-2025-27220 | `lib/cgi`, `ext/cgi/escape` | **does not apply**: not vendored |
| CVE-2023-28756 | `lib/time.rb` | **does not apply**: not vendored |
| CVE-2024-27280 | `ext/stringio` (linked into the test executable of Ruby's suites only) | **does not apply**: not built into vkrgss |
| CVE-2026-41316 | erb | **does not apply**: not vendored |

### The regular expression engine

The rows of the table "Oniguruma 5.9.6" above, for the engine of 1.9.2. Two things of
Ruby 1.9 decide many of them. `re.c` checks the bytes of every pattern before the engine
sees it (`rb_reg_preprocess`: a pattern that is no valid text of its encoding is the error
"invalid multibyte character"), and refuses to match a subject that is no valid text
(`rb_reg_prepare_enc`); and the length functions of the engine take an end. So the defects
of 5.9.6 that read past a character cut off by the end are not reached, but reads of the
byte at the end of a pattern or a subject are: for a Ruby string that byte is its terminator,
and for a pattern in UTF-16LE two to four bytes are read there. The engine was audited
against each row in the tree; "as 5.9.6" means the reason given above holds for this engine
too, after checking that the code is the same or absent.

| Identifier | Code | Built | Reached by | Outcome |
|---|---|---|---|---|
| #16: 65bdf2a, 151b6b4 | `enc/utf_8.c`: `mbc_to_code` reads past a character cut off by the end | yes | no: `re.c` validates patterns and subjects | **does not apply**: the engine is never given a character cut off by the end |
| #17: 8cdbe15 | `regparse.c`: `PFETCH` steps past the end of the pattern | yes | no, as #16; the reads at the end are the rows of dc0a23e, 68c3955, 5186c7c, 562bf48 | **does not apply** |
| #19: e8e827c | `regparse.c`: `onig_parse_make_tree`, a pattern that ends inside a multibyte character | yes | no: `re.c` gives "invalid multibyte character" first | **does not apply**. The one way past that check, a backslash before a character cut off by the end, was found by the target `regex192` and is fixed by patch 25 |
| #18: 44ed4a8 | `enc/utf_8.c`: `USE_INVALID_CODE_SCHEME` | yes | — | **does not apply**: as 5.9.6 |
| #20: dc0a23e | `regparse.c`: `fetch_token` reads after `\k`, `\g`, `\p{` at the end of the pattern (`(?` tests the end in 1.9.2) | yes | data and script | **backported**: patch 34, with the warning and the token that reading the terminator gave |
| #21: 1ca27ef | `regparse.c`: `parse_char_class`, a wide character after a range and `-` taken as one byte | yes | data and script | **backported**: patch 26; found again by the target `regex192` |
| 68c3955 | `regparse.c`: `parse_enclose`, `(?<` at the end | yes | data and script | **backported**: patch 34, with the error that reading the terminator led to ("group name is empty") |
| 5186c7c | `regparse.c`: `fetch_token_in_cc`, `[\p{` at the end | yes | data and script | **backported**: patch 34 |
| 562bf48, 162cf91 | `regparse.c`: `fetch_name_with_level`, `\k<a+`, `\k<a+1` and `\k<-` at the end | yes | data and script | **backported**: patch 34, with the error "invalid group name" as before |
| 8293fd4 | `regparse.c`: `fetch_range_quantifier` | yes | — | **does not apply**: as 5.9.6 |
| 7fe8c3f | `regcomp.c`: `get_min_match_length`, `get_max_match_length` recurse without end on groups that refer to each other, as in `()(\4)()(\2)` | yes | data and script | **backported**, differently: patch 27 stops at a back-reference that is already being measured, with 0 as the shortest and no limit as the longest length; found by the target `regex192`. Upstream rewrote the functions |
| 9be5a03 | `regparse.c`: `fetch_escaped_value` | yes | — | **does not apply**: as 5.9.6 |
| 9574bc1 | `regcomp.c`: `distance_value` indexes a table with a distance that does not fit a signed number | yes | data and script: nested repeats such as `(?:(?:(?:a{0,100000}){0,100000}){0,100000})bc` end the process when compiled | **backported**: patch 36 |
| #27: f37dfad | `regparse.c`: `parse_subexp`, use after free on `(\|` | yes | — | **does not apply**: 1.9.2 frees those nodes in an order that frees each once (traced for `(|`; the upstream diff was not compared) |
| #29: da316ab | `regcomp.c`: `renumber_by_map` skips the target of an anchor | yes | — | **does not apply**: Ruby fixed it in this tree, which renumbers the targets of look-ahead and look-behind |
| #40: 6b68ebe, dacfaba; CVE-2019-16163 (#147: 4097828) | `regparse.c`: no limit on the nesting the parser recurses into | yes | data and script | **left**: as 5.9.6 |
| #43: 5a8a78b; #61: e905508 | `regcomp.c`: captures inside a recursive call | yes | — | **does not apply**: as 5.9.6 |
| CVE-2017-9226 (#55: b4bf968, f015fbd) | `regparse.c`: `next_state_val`, a value above 255 set in the bit set of a class | yes | data and script: a code point above 255 taken for a byte (#21), or an octal value above 255 in a single-byte encoding, which reaches the engine only through an overflow of the octal escape in `re.c` | **backported** in part: patch 26 returns "invalid code point value" for such a value. The error "too big number" for `\400` to `\777` is not taken: `re.c` refuses those escapes already, and the overflowing form is accepted by the original |
| CVE-2017-9225 (#56: 166a6c3) | `unicode_unfold_key.c` | no | — | **does not apply**: a file of 6.x |
| CVE-2017-9224 (#57: 690313a) | `regexec.c`: `match_at`, `OP_EXACT1` reads the subject before it checks its end | yes | data and script | **backported**: patch 35 |
| CVE-2017-9227 (#58: 9690d3a) | `regexec.c`: `forward_search_range`, `reg->dmin` past the end | yes | data and script | **backported**: patch 35. The same check in `backward_search_range` is patch 37 (see CVE-2024-27282) |
| CVE-2017-9229 (#59: b690371) | `regexec.c`: `forward_search_range`, `reg->dmax` before the start | yes | data and script | **backported**: patch 35 |
| CVE-2017-9228 (#60: 3b63d12, ddbf556) | `regparse.c`: `next_state_val`, a range that starts with a class reads a value that was not set (`[\d-z]`) or that of the character before the class (`[a\d-z]`) | yes | data and script | **left**: no memory outside the variable is touched, and the fix of patch 1 of Oniguruma 5.9.6 makes the error of RGSS1's runtime out of a class that the original runtime of RGSS3, this release, accepts (`[a\d-z]` is `a-z` and the digits there). Listed in "Follow-ups" of the change |
| 00d67cc | `regparse.c`: `add_ctype_to_cc_by_range` | yes | — | **does not apply**: as 5.9.6 |
| ab17bd4 | `regparse.c`: `str_reduce_to_single_byte_code` | no | — | **does not apply**: 6.x |
| #84: 34c8cdd | `regexec.c`: `match_at`, the back-reference with a nesting level after an empty match | yes | data and script | **backported**: patch 35 |
| #87: 850bd9b, 410f591 | `regerror.c`: `onig_error_code_to_str` | yes | — | **does not apply**: as 5.9.6; the relative back-references of 1.9.2 fail with an error that takes no name |
| #121: 396a757, ea36d81 | `regparse.c`: callouts | no | — | **does not apply**: 6.x |
| 6fe7b7d | `regexec.c`: `sunday_quick_search` | no | — | **does not apply**: 6.x |
| #139: a8ed18b | `regexec.c`: `match_at`, `OP_MEMORY_END_PUSH_REC` uses a stack pointer after the push that can move the stack | yes | data and script | **backported**: patch 35 |
| CVE-2019-13224 (0f7f61e) | `regext.c`: `onig_new_deluxe` | no | — | **does not apply**: the function is declared, not defined, and never called |
| CVE-2019-13225 (c509265) | `regcomp.c`: `compile_bag_node` | no | — | **does not apply**: 6.x |
| 70e9153 | `regenc.c`: `onigenc_mb2_code_to_mbclen`, EUC-KR | yes | no | **does not apply**: no built encoding calls it |
| 6d248fe | `regexec.c`: `match_at`, `OP_CCLASS_MB` reads the subject before it checks its end | yes | data and script | **backported**: patch 35 |
| 1cb2c31 | `regexec.c`: `match_at`, the four back-reference opcodes after an empty match | yes | data and script | **backported**: patch 35 |
| 7a53a28 | `regparse.c`: `parse_exp`, validity of a string of raw bytes | no | — | **does not apply**: 6.x; `re.c` validates the pattern |
| 8a1c05c | `regexec.c`: the empty check of a look-ahead | no | — | **does not apply**: 6.x |
| CVE-2019-19246 (d3e4029) | `regexec.c`: `str_lower_case_match` reads past the pattern's text | yes | data and script | **backported**: patch 35 |
| #156: 15c4228, adf0fff | `regcomp.c`, `regexec.c` | part | — | **does not apply**: as 5.9.6 |
| CVE-2019-19204 (#162: 6eb4aca) | `regparse.c`: `fetch_range_quantifier` | yes | no | **does not apply**: as 5.9.6 |
| CVE-2019-19203 (#163: aa0188e) | `enc/gb18030.c` | no | — | **does not apply**: not vendored |
| CVE-2019-19012 (#164: 0463e21, db64ef3, bfc36d3, 778a43d, b6cb758) | `regexec.c`: `search_in_range` | no | — | **does not apply** as published, as 5.9.6. The backward search reads the byte after the subject, its terminator: **left**, as 5.9.6 |
| 8d37d84 | `regparse.c`: `onig_reduce_nested_quantifier` | yes | — | **does not apply**: as 5.9.6 |
| #167: 603f4db | `regexec.c`: `MATCH_AT_ERROR_RETURN` | no | — | **does not apply**: 6.x |
| e440706 | `regparse.c`: `parse_exp`, use after free | no | — | **does not apply**: 6.x |
| #169: 2ce691e, #173: 093d296, #175: 56182c7, #177: 498d8f7, 12215ca | `regcomp.c`: `node_char_len1`, `check_node_in_look_behind`, `optimize_nodes`, `tune_look_behind`, `get_tree_tail_literal` | no | — | **does not apply**: 6.x |
| oss-fuzz 22744: 87423b9 | `regparse.c`: `backref_rel_to_abs` | no | — | **does not apply**: 6.x (1.9.2 has a macro of that name that only adds) |
| oss-fuzz 22925: 86f9fc7 | `enc/euc_jp.c`: `code_to_mbc` | no | — | **does not apply**: EUC-JP is not in this tree |
| oss-fuzz 25893: a4aaa67 | `enc/unicode.c`: `onigenc_unicode_get_case_fold_codes_by_str` | yes | — | **does not apply**: as 5.9.6 |
| CVE-2020-26159 (#207: cbe9f8b, 8155473, 3603d78); #207: 2493154, 278a172 | `regcomp.c`, `regparse.c` of 6.x | no | — | **does not apply**: as 5.9.6 |
| #206: c74dc87, 38b02f9, ed4a253, 47d1654, 2b7b941; f29e9ff, 9d9da5a | `regexec.c`, `regparse.c`, `regcomp.c` | part | — | **does not apply**: as 5.9.6 (the commits were not compared one by one) |
| 4d3863b | `regparse.c`: the Python syntax | no | — | **does not apply** |
| #251: 1877295; #265: 2553552; #267: e6aef5e | `regparse.c` of 6.x | no | — | **does not apply** |
| CVE-2019-16161 (Onigmo 9827d5a) | `regparse.c`: `fetch_token`, a null pointer passed to `onig_error_code_to_str` | yes | no, as far as established: the trigger is `(?R` and `(?0` of the Perl syntax, which Ruby does not select | **does not apply** |
| CVE-2019-16162 (Onigmo d4cf99d) | `regenc.c`: `onigenc_single_byte_code_to_mbclen` gives 1 for a code above 255, which `parse_char_class` then reads past | yes | data and script | **left** as code, not reached: patch 26 (CVE-2017-9226) makes a code point above 255 in a class of a single-byte pattern the error "invalid code point value", checked on an interpreter built from this tree, where the release without it ends the process |
| Oniguruma 5.9.6, patches 6 and 8 | `regcomp.c`, `enc/unicode.c`: the lengths of a piece of literal text that ends inside a character | yes | no: no piece of a validated pattern ends inside a character | **does not apply** |
| Oniguruma 5.9.6, patch 7 | `regexec.c`: `OP_PUSH_OR_JUMP_EXACT1`, `OP_PUSH_IF_PEEK_NEXT` read the subject before they check its end | yes | data and script | **backported**: patch 35 |
| Oniguruma 5.9.6, patch 5 | `regparse.c`: `PINC` | yes | no, as #17 | **does not apply** |

### Found by the sanitized run and the fuzz targets

The sanitized run of the suites and the first runs of the targets `marshal192`, `parse192`
and `regex192` (change `fuzz-and-review-ruby-1-9-2`, "Follow-ups" of its `design.md`) found
the rows above that say so and these, which no published fix names. Each is a recorded patch
of `third_party/ruby-1.9.2/PATCHES.md`, and the input of each that a target found is a test
(`crates/vkrgss-ruby192-sys/fuzz/fixed/`).

| Code | Defect | Found by | Patch |
|---|---|---|---|
| `util.c`: `nrv_alloc` | the conversion `%a` of 0, infinity and NaN writes the terminator one byte past its buffer | the sanitized run | 21 |
| `time.c`: `time_mload` | a dumped Time with a month above 12 (four bits hold up to 16) indexes the tables of the months past their end | `marshal192` | 22 |
| `parse.y`: the rule of a regexp literal | after an interpolation the next pieces of text were joined to a node of the list taken for a string, as in `/#$y #{"s"}#{"t"}/`, a valid script: a null pointer is read | `parse192` | 23, 24 |
| `parse.y`: `assign_in_cond` | the value of an assignment in a condition is a null pointer after a syntax error in it (`!w=%w[`) | `parse192` | 23, 24 |
| `re.c`: `unescape_nonascii` | a backslash before a character cut off by the end of the pattern passed the check of the pattern, and the engine read past it | `regex192` | 25 |
| `gc.c`: the marking of `NODE_COLON3` | the name in the node of `::Name` was followed as a pointer; the name of a local variable, which `class ::c` leaves there after its syntax error, ended the process at the next collection | `parse192` | 28 |
