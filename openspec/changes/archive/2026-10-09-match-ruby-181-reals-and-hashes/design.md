# Design

## Context

- Measured with a probe script in the original runtime (`RGSS103J.dll`, `RUBY_VERSION` 1.8.1)
  under Wine and in vkxp. The values in the spec are those measurements.
- `random.c` after `match-ruby-181-random`: `rb_genrand_real` is the 53-bit `genrand_res53`.
  The original's twelve values of `rand(1000)` after `srand(1234)` equal
  `int(1000 * (output / 2^32))` for the first twelve 32-bit outputs of MT19937 initialised by
  `init_genrand(1234)`, and its next `rand` equals the thirteenth output divided by 2^32.
- `string.c` (1.8.7-p374): `rb_str_hash` starts from a `hash_seed` that `Init_String` draws
  from the generator, and mixes differently. The original's values equal, in C on a 32-bit
  build with signed `char`: `int key = 0; while (len--) key = key*65599 + *p++;
  key = key + (key>>5);`, returned through `INT2FIX` (which drops the top bit).
- `numeric.c` (1.8.7): `flo_hash` mixes with XOR. The original's `1.5.hash == 53405` equals
  `hash += c[i] * 971` over the eight bytes as signed `char`, then `if (hash < 0) hash = -hash`.
- `st.c` needs no change: hashes with integer keys already iterate as in the original.
- Symbols created by scripts get other numbers than in the original (16217 against 17489 for
  the first new one in the probe), because the two versions intern a different number of names
  at start-up.

## Decisions

- Patch `rb_genrand_real` to `genrand_int32() * (1.0 / 4294967296.0)`. Other users of the
  53-bit real inside the interpreter, if any, take the same function.
- Patch `rb_str_hash` to the formula above with explicit 32-bit types (`int32_t`, `signed
  char`), so that the 64-bit build computes what the 32-bit original computes, and make
  `rb_str_hash_m` return the value with the top bit dropped as a 32-bit `INT2FIX` would. Remove
  the seeding in `Init_String`.
- Patch `flo_hash` likewise with explicit types.
- Found while implementing: 1.8.7's `rb_any_hash` in `hash.c` drops the top bit of every key's
  hash before the table takes it, and 1.8.1 does not. The measured iteration order (`pdef`,
  whose hash has differing top bits) is only reached when a string key hands all 32 bits to
  the table, so `hash.c` gets a fourth patch for string keys, recorded like the others.
- Record the three patches in `PATCHES.md` with patch files and manifest hashes; adjust
  upstream tests that pin the old values, with a comment, and list them.

## Risks / Trade-offs

- [Removing the hash seed re-opens the hash-flooding weakness the seed was added against] →
  Accepted: scripts are the game's own, and the original has the same property.

## Follow-ups

- Symbol numbering, and with it the iteration order of hashes with symbol keys.
