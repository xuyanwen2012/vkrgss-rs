# Design

## Context

The study, its patch, and the scripts that reproduce the three errors are in
`../../vkxp/target/p1-gc-aarch64/` (read only): `REPORT.md`, `fix.patch`,
`bignum-int-gc-guard.patch`, `marshal-userdef-gc-guard.patch`, `repro/t*.rb`. With the
patch, `cargo test -p vkrgss-ruby192-sys --test upstream_stress --locked` passes on the
Jetson (704 s); without it the four files fail. The flow has no aarch64 machine.

## Decisions

- **The two patches.** Take them from the study, with comments that say `vkrgss:` (the
  study's comments say `vkxp:`, the former name), as patches 41 `bignum-int-gc-guard` and 42
  `marshal-userdef-gc-guard` (or the next free numbers). `bignum.c` already carries patch 40,
  so the patched checksum of entry 40 changes too, as the manifest test requires. The hunk of
  the study in a branch that is not compiled on these targets may be dropped; say so.
- **Audit.** Read `bignum.c` for every function that takes `BDIGITS(x)` (or `RBIGNUM_DIGITS`)
  of an argument and then calls something that can allocate (`bignew`, `rb_big_clone`,
  `bigtrunc` on a new object, `rb_int2big`, `rb_funcall`, anything that can raise) before the
  last use of the pointer, without another use of `x` itself afterwards; likewise `RSTRING_PTR`
  of a string held only by that pointer across an allocation in `marshal.c`, `time.c`,
  `string.c`, `pack.c`, `sprintf.c`. Add `RB_GC_GUARD` where the pattern is real. List every
  place found, guarded or judged safe with the reason, under a heading "Audit" here. Do not
  rewrite code beyond the guards.
- **Tests.** In `vkrgss-ruby192-sys`, a test file (or cases in an existing one) that runs the
  reductions `repro/t2.rb`, `t4.rb`, `t5.rb`, `t6.rb` under `GC.stress = true` through the
  interpreter and checks: `File.utime` with `Time.local(2000,1,1)` succeeds; `Marshal.dump`
  of `Time.at(0, Rational(1,3))` and of `Time.at(0, 0.123)` returns a string that loads back
  to an equal time; the two DST times of `t6.rb` give the same results as without stress; and
  `TIMET_MAX + 1` style arithmetic: `(2**63 - 1) + 1 == 2**63` and `2**63 - 1 - (-1)` under
  stress. Use a file the test creates instead of `t1.rb` for `File.utime`.
- **1.8.7.** Its Bignums are not embedded and its marshal already has the guard; the audit
  of 1.8.7 is out of scope.

## Out of scope

Compiling with flags that change register allocation; Ruby 1.8.7.

## Follow-ups

None yet.
