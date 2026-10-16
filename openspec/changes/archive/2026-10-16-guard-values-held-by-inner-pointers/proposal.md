# Guard values that only an inner pointer holds (Ruby 1.9.2)

## Why

The first stressed run of Ruby 1.9.2's suite on aarch64 (the owner's Jetson Orin Nano, gcc
13.3, 2026-10-16) failed in four files that pass on x86_64: `test_file_exhaustive.rb`
(`test_utime`, "time out of system range"), `test_marshal.rb` and `test_time.rb` ("can't
dump anonymous ID 0"), and `test_time_tz.rb` (wrong times at two DST transitions). A study
on the Jetson (`vkxp/target/p1-gc-aarch64/REPORT.md` of the workspace) found two places where
the only live reference to an object is a pointer into it, which the conservative collector
does not count as a root:

- `bignum.c`, `bigadd_int` and `bigsub_int`: `xds = BDIGITS(x)` is taken, `bignew`
  allocates, and `xds` is read afterwards. A Bignum of two digits is embedded, so `xds` is
  `x + 0x10`. gcc 13 on aarch64 keeps only `xds` across the call; under stress `x` is
  collected and `z` takes its slot, so the sum reads zeros (`add(TIMET_MAX, 1)` gave 1, which
  made every time after 1970 "out of system range").
- `marshal.c`, `w_object`, the branch of `_dump`: after `has_ivars(v, ivtbl2)` the string
  `v` is used only through `RSTRING_PTR(v)` and `ivtbl2`; `w_class` allocates, `v` is
  collected with its generic instance variables, and `w_ivar` walks the freed table. The tree
  of Ruby 1.8.7 has exactly this guard already (its patch 23).

x86_64 passes only because its code generation happens to keep the object itself in a
register. P1 (ARM Linux) needs the stressed suite to pass on aarch64.

## What Changes

- `RB_GC_GUARD` where the object is held only through an inner pointer across an allocation:
  the two places above, and any other place of the vendored tree with the same pattern that
  an audit of `bignum.c`, `marshal.c`, `string.c`, `time.c` and the other files reached from
  `Marshal`, `Time` and arithmetic finds. Each as a recorded patch.
- Tests that run the reductions found on the Jetson under `GC.stress` and expect the right
  results. They pass on x86_64 before the patch too; they are the regression tests for
  aarch64, where the owner's machine and the CI job `gate-aarch64` run them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby-1-9-2`: values held only by an inner pointer are kept across allocations.

## Impact

`third_party/ruby-1.9.2` (recorded patches only), the tests of `vkrgss-ruby192-sys`,
`docs/vendored-security.md` if it lists collector fixes, `CHANGELOG.md`. A game sees a
difference only where the collector freed a live object, which was wrong.
