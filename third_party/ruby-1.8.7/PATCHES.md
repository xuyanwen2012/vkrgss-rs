# Local patches to Ruby 1.8.7-p374

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

## 1. zlib-crc-table-64bit

- File: `ext/zlib/zlib.c`
- Patch: `patches/zlib-crc-table-64bit.patch`
- Patched SHA-256: `a0f5f9000fdd78ad0ea6cc0404fae44f26ae89eea16e8070222a77e8b28e4efc`
- Reason: `Zlib.crc_table` read zlib's table of 32-bit values through `const unsigned long *`.
  Where `long` has 64 bits that returns wrong values and reads past the end of the table. The
  pointer type becomes zlib's own `const z_crc_t *`.

## 2. random-ruby-181-sequence

- File: `random.c`
- Patch: `patches/random-ruby-181-sequence.patch`
- Patched SHA-256: `e305120d31bf3157f39ced693fafcc338660877186c09a2975ea632657e6e6da`
- Reason: the original runtime embeds Ruby 1.8.1, and a game is a deterministic function of
  `rand`, so the same seed has to give the same sequence. As in 1.8.1, `srand` initialises the
  generator with `init_genrand` from one 32-bit integer and returns the previous such integer,
  and `rand(n)` for a Fixnum `n` (or a Float in the range of `long`) is the generator's real
  number (see patch 4) times the absolute value of `n`, truncated. Differences that remain: a seed of more than 32 bits
  counts by its low 32 bits, where 1.8.1 raises `RangeError`; `srand` without an argument
  still draws from `/dev/urandom`, time and process id; a Bignum limit keeps 1.8.7's code,
  where 1.8.1 calls `rb_big_rand` with an array of reals.

## 3. test-rand-ruby-181-sequence

- File: `test/ruby/test_rand.rb`
- Patch: `patches/test-rand-ruby-181-sequence.patch`
- Patched SHA-256: `2ea879fd106c98719086771bdc4ea2d9c543d7ea887078b69bd478772cf3b6b4`
- Reason: follows patch 2. The thirteen tests whose limit is a Fixnum on a 64-bit build
  (`test_mt`, `test_0x3fffffff`, `test_0x40000000`, `test_0x40000001`, `test_0xffffffff`,
  `test_0x100000000`, `test_0x100000001`, `test_rand_0x100000000`, `test_0x1000000000000`,
  `test_0x1000000000001`, `test_0x3fffffffffffffff`, `test_neg_0x10000000000`,
  `test_neg_0x10000`) pinned 1.8.7's sequences and now pin the 1.8.1 ones, computed with an
  independent MT19937. `test_0x4000000000000000` and `test_0x4000000000000001` are unchanged.
  Patch 8 pins them again.

## 4. random-ruby-181-real

- File: `random.c`
- Patch: `patches/random-ruby-181-real.patch`
- Patched SHA-256: `e305120d31bf3157f39ced693fafcc338660877186c09a2975ea632657e6e6da`
- Reason: applies after patch 2. Measured on the original runtime, 1.8.1 builds each real
  number from one 32-bit output of the generator divided by 2^32, where 1.8.7's
  `rb_genrand_real` takes two outputs for 53 bits and so consumes the sequence twice as fast.
  `rb_genrand_real` becomes the 32-bit form; `rand`, `rand(n)` and the other callers inside
  the interpreter (`Array#shuffle!`, `Array#choice`) all go through it.

## 5. string-hash-ruby-181

- File: `string.c`
- Patch: `patches/string-hash-ruby-181.patch`
- Patched SHA-256: `ac3311aebe253de3df594e123a04372437bd46f110945bc711ef821a00bccfeb`
- Reason: the iteration order of a hash with string keys follows from `String#hash`, and
  scripts depend on that order. 1.8.7 mixes differently and starts from a seed drawn at
  start-up, so the order differed from the original and from run to run. `rb_str_hash` is the
  1.8.1 function (`key = key*65599 + byte` over signed bytes, then `key + (key>>5)`) in
  explicit 32-bit types, so that the 64-bit build computes what the 32-bit original computes;
  `String#hash` drops the top bit as a 32-bit `INT2FIX` does; the seed is gone. This re-opens
  the hash-flooding weakness the seed was added against, which the original has as well.

## 6. hash-string-key-all-bits

- File: `hash.c`
- Patch: `patches/hash-string-key-all-bits.patch`
- Patched SHA-256: `738525e4393857e58c8d329222d74c0b26608f79977b93a8c622d867aa81559a`
- Reason: follows patch 5. 1.8.7's `rb_any_hash` drops the top bit of every key's hash before
  the table takes it; 1.8.1 hands a string's hash to the table whole. With the bit dropped,
  a key whose two top hash bits differ (`"pdef"`, for one) lands in another bin than in the
  original, and the iteration order differs. String keys now return all 32 bits.

## 7. float-hash-ruby-181

- File: `numeric.c`
- Patch: `patches/float-hash-ruby-181.patch`
- Patched SHA-256: `5ca01d96bdef7dc5cee9ae7ebc61aa54e59aca9790b820414805394bdfdc8f4d`
- Reason: 1.8.7's `flo_hash` mixes the bytes of the double with XOR. As in 1.8.1, it is the
  sum of the eight bytes, each taken as a signed 8-bit value and multiplied by 971, made
  non-negative, in explicit types.

## 8. test-rand-ruby-181-real

- File: `test/ruby/test_rand.rb`
- Patch: `patches/test-rand-ruby-181-real.patch`
- Patched SHA-256: `2ea879fd106c98719086771bdc4ea2d9c543d7ea887078b69bd478772cf3b6b4`
- Reason: follows patch 4 and applies after patch 3. The thirteen tests that patch 3 lists
  pinned sequences built from the 53-bit real and now pin those built from one 32-bit output
  per number, computed with an independent MT19937.

## 9. test-string-hash-not-random

- File: `test/ruby/test_string.rb`
- Patch: `patches/test-string-hash-not-random.patch`
- Patched SHA-256: `ca5cb0f891a1153e6dd0dc44e507ee30fde1ccb226a1ae260c22ab16f2fa4a88`
- Reason: follows patch 5. `test_hash_random` asserted that `'abc'.hash` differs between
  processes; without the seed it asserts that every process computes the same value.

## 10. class-redefinition-ruby-181

- File: `eval.c`
- Patch: `patches/class-redefinition-ruby-181.patch`
- Patched SHA-256: `5305d4212e520e8f167962b8843338384608117760f310f613c2187973d01579`
- Reason: measured on the original runtime, a `class` statement that names an existing class
  with an explicit superclass other than that class's own defines a new class and binds the
  constant to it; the old class object and its instances stay as they were. 1.8.7 raises
  `TypeError` ("superclass mismatch"), and a game whose scripts redefine a class this way did
  not start. As in 1.8.1, `NODE_CLASS` goes on to the code that defines a class for an
  undefined constant. The constant is removed first, so that no "already initialized
  constant" warning is printed; the original prints none. No upstream test pinned the
  `TypeError`.

## 11. symbol-ids-ruby-181

- File: `parse.y`
- Patch: `patches/symbol-ids-ruby-181.patch`
- Patched SHA-256: `ed38d409059cbbcee58ec7991446f41406f87123b0dc1ae2db9bc4b2bcc0d5d5`
- Reason: a symbol's number decides the order of instance variables and of hashes with
  symbol keys, and so the bytes of a save file. 1.8.7 and vkxp's binding intern another set of
  names at start-up than the original runtime's Ruby 1.8.1, so every name a game creates had
  another number. `rb_intern` now gives a name of the measured table
  (`crates/vkxp-ruby-sys/data/symbols.startup.txt`, the 2002 symbols the original has before
  its first game script; the build generates `vkxp_symbol_table.h` from it) the number it has
  there, and calls `rb_bug` when the scope bits it derives from the name disagree with the
  table. Any other name interned before the host calls `ruby_vkxp_mark_game_scripts` is
  numbered from a provisional counter starting at 2^24. From the mark on, new names and the
  parser's internal identifiers continue the original's sequence (counter 2024, the number
  16193 for a local variable), and the first `rb_intern` of a provisional name advances that
  sequence once and keeps the name's number. An executable that never calls the mark, such
  as the one of the upstream suites, numbers every name outside the table provisionally.

## 12. symbol-ids-ruby-181-generated

- File: `parse.c`
- Patch: `patches/symbol-ids-ruby-181-generated.patch`
- Patched SHA-256: `bef97ac617211c1ab9eff4daf7d9fe9bd3e66f875010ebdbecd8e2d140b0dfc7`
- Reason: follows patch 11. `parse.c` is the release's pre-generated parser and the file the
  build compiles; the changed code is in the part of `parse.y` that the generator copies
  verbatim, so the same three hunks are applied to it by hand, at an offset of 5947 lines.

## 13. oniguruma-engine

- File: `re.c`
- Patch: `patches/oniguruma-engine.patch`
- Patched SHA-256: `9609a1b0fbce194822cee13ae4115736fd516bbfcf54fc102e967cb7d87e96d2`
- Reason: the original runtime's Ruby 1.8.1 is built with the Oniguruma regular expression
  engine, and scripts use what only that engine accepts (a character class inside another,
  named groups, look-behind, possessive quantifiers). `re.c` now calls Oniguruma 5.9.6
  (`third_party/oniguruma-5.9.6/`) through its GNU-compatible interface, and `regex.c` is no
  longer built. A pattern is allocated with `re_alloc_pattern`, which gives it the encoding
  that `re_mbcinit` set last from `$KCODE` or the expression's own flag, and compiled with
  `re_compile_pattern`, whose message becomes the error's. Because an Oniguruma pattern keeps
  its encoding, a change of `$KCODE` or `$=` compiles a new pattern in place of the old one.
  The syntax is Oniguruma's Ruby syntax less five options that the original's older Oniguruma
  does not show: measured there, `\h`, `\p{...}` and `{,n}` are literal text and `{n}?` is a
  reluctant interval. A backward search (`rindex`, `start_with?`) tries a match at each
  character boundary from the start position down, as the former engine did, because
  Oniguruma's own backward search rejects a match that reaches beyond the character at the
  start position. A search that fails with an error raises with Oniguruma's message in place
  of "Stack overflow in regexp matcher". `re_mbc_startpos`, which `parse.y` takes from
  `regex.c`, is defined here on Oniguruma's encoding functions.

## 14. oniguruma-engine-header

- File: `re.h`
- Patch: `patches/oniguruma-engine-header.patch`
- Patched SHA-256: `dfd1988bd6289f64ee6a7bdce89bf4d74c716fa5c12d6ef9805622ec3d8811ef`
- Reason: follows patch 13. `re.h` includes `vkxp_regex.h` in place of `regex.h`. That header
  is `crates/vkxp-ruby-sys/compat/vkxp_regex.h`: it includes Oniguruma's `oniggnu.h` and
  provides the `MBCTYPE_*` names, `re_copy_registers`, and `ismbchar` and `mbclen` on the
  encoding that `re_mbcinit` set, so that `string.c` and the other files that use them are
  unchanged.

## 15. oniguruma-engine-parser

- File: `parse.y`
- Patch: `patches/oniguruma-engine-parser.patch`
- Patched SHA-256: `ed38d409059cbbcee58ec7991446f41406f87123b0dc1ae2db9bc4b2bcc0d5d5`
- Reason: follows patch 14 and applies after patch 11. The parser includes `regex.h` itself,
  for `ismbchar` and `mbclen`; it includes `vkxp_regex.h` instead.

## 16. oniguruma-engine-parser-generated

- File: `parse.c`
- Patch: `patches/oniguruma-engine-parser-generated.patch`
- Patched SHA-256: `bef97ac617211c1ab9eff4daf7d9fe9bd3e66f875010ebdbecd8e2d140b0dfc7`
- Reason: follows patch 15 and applies after patch 12. The same line of the pre-generated
  parser, changed by hand.

## 17. test-regexp-oniguruma

- File: `test/ruby/test_regexp.rb`
- Patch: `patches/test-regexp-oniguruma.patch`
- Patched SHA-256: `c0eeadd74feec67bfc0fff61e6952b8ddc18f7b844a72e7b95732bdb59e8cc51`
- Reason: follows patch 13. Three cases pinned behaviour of the former engine and now pin
  Oniguruma's. In `test_parse`, `a++` was an error and is a possessive quantifier. In
  `test_unescape`, `\x` without a digit was an error and is accepted. In
  `test_parse_curly_brace`, `(?!x){0,1}` matched and is an error ("target of repeat operator
  is invalid").

## 18. asan-conservative-scan

- File: `gc.c`
- Patch: `patches/asan-conservative-scan.patch`
- Patched SHA-256: `74048030ef7c2198d7299ea1deeb9161e1fb44a06504a5f6f5704067bf34192e`
- Reason: for the sanitized run of the upstream suites (`tests/sanitized.rs` of
  `vkxp-ruby-sys`). The collector is conservative: `mark_locations_array` reads every word of
  the machine stack and of the saved registers, the red zones that AddressSanitizer puts
  around local variables included, and the sanitizer stopped the first collection. The
  function carries the compiler's `no_sanitize_address` attribute. Without the sanitizer the
  attribute changes nothing.

## 19. asan-thread-switch

- File: `eval.c`
- Patch: `patches/asan-thread-switch.patch`
- Patched SHA-256: `5305d4212e520e8f167962b8843338384608117760f310f613c2187973d01579`
- Reason: for the same run. A thread switch (and a continuation) copies the machine stack to
  the heap in `rb_thread_save_context` and back in `rb_thread_restore_context_0`, red zones
  included, and the sanitizer's `memcpy` stopped the first switch. Under AddressSanitizer the
  copied range is declared addressable before each copy
  (`__asan_unpoison_memory_region`); the frames that are live at a switch lose their red
  zones, so the run can miss a stack overflow in them and cannot report a false one. In any
  other build the macro is empty.

## 20. make-exception-one-argument

- File: `eval.c`
- Patch: `patches/make-exception-one-argument.patch`
- Patched SHA-256: `5305d4212e520e8f167962b8843338384608117760f310f613c2187973d01579`
- Reason: found by the sanitized run. With one argument that is not a string, as in
  `raise SomeError`, `rb_make_exception` passed `argv[1]` to a call that takes no argument
  from it: a read of one word past the end of the argument array. The value was never used.
  The call with no argument no longer reads it.

## 21. intern-empty-name

- File: `parse.y`
- Patch: `patches/intern-empty-name.patch`
- Patched SHA-256: `ed38d409059cbbcee58ec7991446f41406f87123b0dc1ae2db9bc4b2bcc0d5d5`
- Reason: found by the sanitized run. `rb_intern` of a name that consists of `=` alone
  interns the name without its last character, the empty string, and for that one tested
  `name[-1]`, the byte before the buffer; had that byte been `=`, a copy of length -1 would
  have followed. The test is made only for a name that has a last character.

## 22. intern-empty-name-generated

- File: `parse.c`
- Patch: `patches/intern-empty-name-generated.patch`
- Patched SHA-256: `bef97ac617211c1ab9eff4daf7d9fe9bd3e66f875010ebdbecd8e2d140b0dfc7`
- Reason: follows patch 21. The same line of the pre-generated parser, changed by hand.

## 23. marshal-gc-guard

- File: `marshal.c`
- Patch: `patches/marshal-gc-guard.patch`
- Patched SHA-256: `f11ad0ab56d01ede86e4ba14b4c49557db77faed63b6b55ad7bac2e52ef77d9d`
- Reason: found by the collection stress test of `vkxp-binding` on a build with gcc 13.3,
  which read `[980748292, 1650545674, ...]` where a dumped `Table` has its sizes. `w_object`
  hands `RSTRING(v)->ptr` of the string a `_dump` method returned to `w_bytes` and does not
  use `v` again; the collector is conservative, so once the compiler has dropped `v` a
  collection inside `w_bytes` frees the string and the bytes copied afterwards are whatever
  the allocator left there. `RB_GC_GUARD` after the last use of the buffer keeps the value
  on the stack, as later Ruby versions do. Guarded the same way: the class path in
  `w_class`, `w_uclass` and the two cases that dump a class or a module; the name that
  `r_symreal` passes to `rb_intern`, which copies it after an allocation, so that a symbol
  loaded for the first time during a collection got a name of freed bytes; and the source of
  `marshal_load`, which is otherwise held only by the unmarked `struct load_arg` when it is
  the result of `to_str`.

## 24. pack-gc-guard

- File: `pack.c`
- Patch: `patches/pack-gc-guard.patch`
- Patched SHA-256: `bbd02ec3e5657b9d530ad3443705b1e698a42997ee29eccfe2839a95550179e4`
- Reason: found by the reading that followed patch 23, the same fault. In `pack_pack` the
  directive `w` builds each number in a string `buf` of its own and appends
  `RSTRING(buf)->ptr` to the result with `rb_str_buf_cat`, which grows the result before it
  copies and does not use `buf` again; a collection at that point (growing collects under
  `GC.stress` or when memory is short) frees `buf`. `RB_GC_GUARD(buf)` follows the call.
  Guarded as well, though each is a local whose address `StringValue` has passed on, so
  that a compiler that does not look across files keeps it: the element `from` of the
  directives `a A Z b B h H u m M`, whose buffer is read while the result grows, and the
  format `fmt` of `pack_unpack`, which is read to the end of the loop. These matter only
  for an argument that is not a string and is converted by `to_str` or `to_s`.

## 25. zlib-gc-guard

- File: `ext/zlib/zlib.c`
- Patch: `patches/zlib-gc-guard.patch`
- Patched SHA-256: `a0f5f9000fdd78ad0ea6cc0404fae44f26ae89eea16e8070222a77e8b28e4efc`
- Reason: applies after patch 1; the same reading. `do_deflate` and `do_inflate` pass
  `RSTRING(src)->ptr` to `zstream_run` as their last statement, which allocates the input
  string before it copies from the pointer; `Zlib::Inflate#inflate` and `#<<` do the same
  with `zstream_append_buffer` on a finished stream. `src` is not used afterwards. A string
  argument is held by the caller's arguments; the result of `to_str` for another object is
  held only by the parameter, whose address `StringValue` has passed on. `RB_GC_GUARD(src)`
  follows each call.

## 26. time-mload-old-format-gmt

- File: `time.c`
- Patch: `patches/time-mload-old-format-gmt.patch`
- Patched SHA-256: `8c0f49822f74791f017e98f8851d8c6a124cde8648f1bea7cd4ed9e6c8a6bc6f`
- Reason: found by compiling with the warnings for uninitialised variables (change
  `fix-findings-of-the-first-ci-runs`). `time_mload` reads two formats. For the older one,
  seconds and microseconds with the top bit of the first word clear, it never set `gmt` and
  then stored the indeterminate value in the object, so whether such a `Time` loaded with
  `Marshal.load` was in UTC depended on what the compiler left on the stack. The older format
  has no flag for UTC; `gmt` is 0 for it, which is local time.

## 27. gc-stress-every-nth

- File: `gc.c`
- Patch: `patches/gc-stress-every-nth.patch`
- Patched SHA-256: `74048030ef7c2198d7299ea1deeb9161e1fb44a06504a5f6f5704067bf34192e`
- Reason: applies after patch 18; for the runs under collector stress (change
  `stress-the-collector-and-try-aarch64`). `GC.stress` collects at every allocation, which
  is too slow for whole suites, and a script has to set it. `ruby_vkxp_gc_stress(n)` makes
  the collector run at every n-th allocation instead, counting `rb_newobj`, `ruby_xmalloc`
  and `ruby_xrealloc` together; `vkxp-ruby-sys` calls it at start-up when the environment
  variable `VKXP_GC_STRESS` holds a number of 1 or more. With 0, the initial value, each of
  the three places tests one variable more and nothing else changes. A collection asked for
  while collections are disabled or one is running does not happen, as before.
  `ruby_vkxp_gc_count` counts the collections that ran, for the tests: this Ruby has no
  `GC.count`.

## 28. intern-incomplete-character

- File: `parse.y`
- Patch: `patches/intern-incomplete-character.patch`
- Patched SHA-256: `ed38d409059cbbcee58ec7991446f41406f87123b0dc1ae2db9bc4b2bcc0d5d5`
- Reason: found by the fuzz targets `marshal` and `parse` (change
  `fuzz-the-interpreter-and-review-fixes`). `rb_intern` walks a name character by
  character with the length the lead byte announces. A name that ends in an incomplete
  multibyte character, such as the one byte `\xfa` under `$KCODE = 'U'`, moved the
  pointer past the terminating NUL, and the test that follows read a byte after the end of
  the buffer to decide whether the name has a junk tail. The test is made only while the
  pointer is inside the name. A dumped symbol reaches it through `Marshal.load`, and an
  identifier of a script through the lexer. Input:
  `crates/vkxp-ruby-sys/fuzz/fixed/marshal/symbol-incomplete-character.bin`.

## 29. intern-incomplete-character-generated

- File: `parse.c`
- Patch: `patches/intern-incomplete-character-generated.patch`
- Patched SHA-256: `bef97ac617211c1ab9eff4daf7d9fe9bd3e66f875010ebdbecd8e2d140b0dfc7`
- Reason: follows patch 28. The same line of the pre-generated parser, changed by hand.

## 30. parser-bounds-and-null-operands

- File: `parse.y`
- Patch: `patches/parser-bounds-and-null-operands.patch`
- Patched SHA-256: `ed38d409059cbbcee58ec7991446f41406f87123b0dc1ae2db9bc4b2bcc0d5d5`
- Reason: applies after patch 28; found by the fuzz targets `parse` and `marshal` (change `fuzz-the-interpreter-and-review-fixes`).
  Four defects of the parser on input that is no valid script:
  `is_special_global_name` and `rb_symname_p` stepped over a name by the length its lead
  bytes announce, so a name that ends in an incomplete multibyte character (`$-` and one
  byte `\xf3`) was read past its terminator; both now stop at the terminator
  (`vkxp_char_end`). `yyerror` copies the line of the error for the message: after an
  incomplete multibyte character the lexer's position is past the end of the line, and the
  copy read past the line and the `^` mark was written past the copy; the start, the end
  and the mark are kept inside the line. The rules of `..` and `...` and `assign_in_cond`
  looked at the type of an operand that is a null pointer when the operand had a syntax
  error of its own (`1 .. A::B += 1`, `if g = __FILE__ = 1`), which ends the process; they
  test the pointer first, as later Rubies do. Inputs:
  `crates/vkxp-ruby-sys/fuzz/fixed/marshal/global-name-incomplete-character.bin` and
  `fuzz/fixed/parse/{error-line-incomplete-character,range-null-operand,condition-null-value}.rb`.

## 31. parser-bounds-and-null-operands-generated

- File: `parse.c`
- Patch: `patches/parser-bounds-and-null-operands-generated.patch`
- Patched SHA-256: `bef97ac617211c1ab9eff4daf7d9fe9bd3e66f875010ebdbecd8e2d140b0dfc7`
- Reason: follows patch 30. The same lines of the pre-generated parser, changed by hand.

## 32. regexp-source-incomplete-character

- File: `re.c`
- Patch: `patches/regexp-source-incomplete-character.patch`
- Patched SHA-256: `9609a1b0fbce194822cee13ae4115736fd516bbfcf54fc102e967cb7d87e96d2`
- Reason: applies after patch 13; found by the fuzz targets `regex` and `marshal` (change `fuzz-the-interpreter-and-review-fixes`).
  `rb_reg_expr_str` writes the source of a pattern into `inspect` and into the message of a
  `RegexpError`. It copied a multibyte character, and the character after a backslash, with
  the length the lead byte announces, past the end of a source that ends inside the
  character. Both copies stop at the end of the source. Inputs:
  `crates/vkxp-ruby-sys/fuzz/fixed/regex/error-message-incomplete-character.txt` and
  `fuzz/fixed/marshal/regexp-error-incomplete-character.bin`.
