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
- Patched SHA-256: `0051500303cd272e76cb50cd1bb029716c7ee99074b239f14df8ca0cad75a164`
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
