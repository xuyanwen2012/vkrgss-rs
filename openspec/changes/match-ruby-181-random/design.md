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
