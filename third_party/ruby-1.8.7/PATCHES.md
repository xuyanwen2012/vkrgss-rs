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
- Patched SHA-256: `3328710a77ae9fe5663862e15e13d6e20fb15fdf5bc87b0f6248b2197dbe712d`
- Reason: the original runtime embeds Ruby 1.8.1, and a game is a deterministic function of
  `rand`, so the same seed has to give the same sequence. As in 1.8.1, `srand` initialises the
  generator with `init_genrand` from one 32-bit integer and returns the previous such integer,
  and `rand(n)` for a Fixnum `n` (or a Float in the range of `long`) is the 53-bit real times
  the absolute value of `n`, truncated. Differences that remain: a seed of more than 32 bits
  counts by its low 32 bits, where 1.8.1 raises `RangeError`; `srand` without an argument
  still draws from `/dev/urandom`, time and process id; a Bignum limit keeps 1.8.7's code,
  where 1.8.1 calls `rb_big_rand` with an array of reals.

## 3. test-rand-ruby-181-sequence

- File: `test/ruby/test_rand.rb`
- Patch: `patches/test-rand-ruby-181-sequence.patch`
- Patched SHA-256: `3f40623b4bb8eeb9dcf425be9a1a1ac3bfe9edc37ea9a791bc1cd439c78c5418`
- Reason: follows patch 2. The thirteen tests whose limit is a Fixnum on a 64-bit build
  (`test_mt`, `test_0x3fffffff`, `test_0x40000000`, `test_0x40000001`, `test_0xffffffff`,
  `test_0x100000000`, `test_0x100000001`, `test_rand_0x100000000`, `test_0x1000000000000`,
  `test_0x1000000000001`, `test_0x3fffffffffffffff`, `test_neg_0x10000000000`,
  `test_neg_0x10000`) pinned 1.8.7's sequences and now pin the 1.8.1 ones, computed with an
  independent MT19937. `test_0x4000000000000000` and `test_0x4000000000000001` are unchanged.
