# Round the mathematical functions correctly

## Why

The owner decided on 2026-10-15 ("Floats in saves" in `docs/decisions.md`) that a save is
identical byte for byte across machines and platforms. A save holds floats that `Math` and
`Float#**` computed, and both interpreters take those functions from the C library of the
machine, whose results differ between versions of glibc and between operating systems.
Measured on 2026-10-16 on this machine: on hard-to-round inputs, glibc differs from the
correctly rounded result in 4 to 46 percent of the cases for 18 of the 24 functions below
(`tgamma` by up to 4 units in the last place); only the functions glibc already took from
CORE-MATH (`asinh`, `acosh`, `atanh`, `erf`, `erfc`) and `hypot` agree everywhere.

Only correct rounding gives one result for one input on every platform, independent of the
library's version. CORE-MATH (INRIA, MIT) provides correctly rounded binary64 versions of
every function the interpreters use. A pure Rust library with that guarantee does not exist
today.

## What Changes

- `third_party/core-math`: the binary64 sources of CORE-MATH for the functions the
  interpreters use, at a pinned commit, with a manifest and a test as the other vendored
  trees have.
- Both interpreters call them through a small shim for `sin cos tan asin acos atan atan2
  sinh cosh tanh asinh acosh atanh exp log log2 log10 pow cbrt hypot erf erfc tgamma
  lgamma` (the last four and `log2`, `cbrt` only where the interpreter has them). The
  shim keeps the special inputs, `errno` and the error behaviour of `Math` as they are.
- Both interpreters, and CORE-MATH, are compiled with `-ffp-contract=off`, so that no
  compiler fuses a multiplication and an addition on one architecture and not on another
  (a prerequisite of P1, ARM Linux).
- Test vectors: 5851 inputs (hard cases of CORE-MATH and random ones) with their correctly
  rounded results, checked through `Math` of each interpreter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: correctly rounded mathematical functions; no contraction.
- `embedded-ruby-1-9-2`: the same.

## Impact

`third_party/core-math` (new), `third_party/ruby-1.8.7` and `third_party/ruby-1.9.2`
(recorded patches only), the build scripts and tests of `vkrgss-ruby-sys` and
`vkrgss-ruby192-sys`, `docs/vendored-security.md` (a line on the new tree),
`docs/building.md`, `CHANGELOG.md`, `README`/licence notices where third-party licences are
listed. A game sees different floats only in the last bit, where the C library of this
machine was not correctly rounded; whether the original runtime's results (msvcrt, x87)
agree more or less often is not known, and the comparison with the original is rerun at the
merge.
