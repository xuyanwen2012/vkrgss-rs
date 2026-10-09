# Design

## Context

- `third_party/ruby-1.8.7/random.c` (1.8.7-p374): `rand_init` turns the seed into an array of
  32-bit words and calls `init_by_array`; `rb_f_rand` uses `limited_rand` (masking and retrying
  32-bit outputs) for integer limits and `rb_big_rand`-style limiting for bignums.
- Ruby 1.8.1's `random.c`: `rand_init(seed)` calls `init_genrand(seed)` with an
  `unsigned long` seed and remembers it; `rb_f_rand` computes, for a fixnum limit `max`,
  `val = max * genrand_real(); return LONG2NUM(val);` after taking the absolute value, returns
  `rb_float_new(genrand_real())` for 0 or nil, and for a bignum limit fills an array of
  `genrand_real()` values for `rb_big_rand`. `genrand_real` is the 53-bit `genrand_res53`.
  `srand` without argument seeds from time, process id and a counter.
- The oracle harness (`cargo xtask oracle`) seeds both runtimes with `srand` in its probe; the
  coordinator will confirm the patch against the original after this change.
- Existing patches are recorded in `third_party/ruby-1.8.7/PATCHES.md` with patch files and
  manifest hashes checked by a test.

## Decisions

- Patch `rand_init`, `rb_f_srand` and `rb_f_rand` to the 1.8.1 behaviour for fixnum and float
  arguments. The bignum path keeps 1.8.7's code (PLACEHOLDER: 1.8.1's `rb_big_rand` with an
  array of reals is not reproduced; games do not call `rand` with bignums).
- Tests compute expected values from a small MT19937 written in the test, not from constants
  copied out of anywhere, except the one widely published first real for seed 1.
- Upstream test cases that pin 1.8.7 sequences are changed to the new values with a comment,
  and listed in `PATCHES.md`.

## Risks / Trade-offs

- [The recollection of the 1.8.1 source is wrong in a detail] → The oracle comparison after the
  merge decides; the patch is small and easy to adjust.

## Follow-ups

- A seed of more than 32 bits: 1.8.1 on the original's 32-bit build raises `RangeError` from
  `NUM2ULONG`; the patch takes the low 32 bits instead, as the requirement words it. Whether a
  game could depend on the error is open.
- `srand` without an argument, and the seed at start-up, still come from 1.8.7's
  `/dev/urandom`-based source, reduced to 32 bits, not from 1.8.1's time, process id and
  counter. The value is not reproducible either way; left as it was.
- On a 64-bit build a limit from 2^30 up to 2^62 is a Fixnum and takes the 1.8.1 Fixnum
  formula, while the original's 32-bit build treats it as a Bignum (`rb_big_rand`). Together
  with the Bignum placeholder above, limits of 2^30 and more are not confirmed against the
  original.
