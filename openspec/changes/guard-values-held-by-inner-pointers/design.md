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

## Audit

The patches of the study are recorded as patches 41 and 42. Its first hunk, a guard in the
branch of `bigsub_int` for `SIZEOF_BDIGITS == SIZEOF_LONG`, is left out: `BDIGIT` is
`unsigned int` on every target of vkrgss, so that branch is not compiled.

The rule applied: a guard is added where the object is read through a pointer into it after
a call that can allocate (or run Ruby code, raise, or let another thread run), the function
does not use the object itself afterwards, and nothing else is known to hold it. An object is
known to be held when it is the receiver or an argument of a method that the virtual machine
called (its stack holds them), when it is stored in a marked table or a `volatile` variable,
or when its variable's address was passed to a function of another file (`StringValue` from
`marshal.c`, `time.c`, `pack.c`, `sprintf.c`, `bignum.c`, `rb_scan_args`): the build has no
link-time optimisation, so the variable keeps its stack slot. Within `string.c` that last
reason does not hold, since `rb_string_value` may be inlined there.

Guarded (patches 43 to 45):

- `bignum.c` `bigmul1_balance`: `yds` points into `y` and is read in the loop after
  `bigmul0` allocates; `rb_big_pow` passes its running product, which nothing else holds.
- `bignum.c` `bigdivrem`: `xds` and `yds` are read after `bignew` allocates; without a
  remainder `x` is not used again, and `big_fdiv` passes temporaries from `big_shift`,
  `rb_int2big` and `dbl2big` that it does not use again.
- `bignum.c` `bigand_int`, `bigor_int`, `bigxor_int`: `xds` is read after `bignew`; for a
  negative receiver `x` is a clone that `rb_big_and`, `rb_big_or` and `rb_big_xor` hold in a
  `volatile` variable, but they call these in tail position, which may drop their frame.
- `string.c` `rb_str_sub_bang`: `rp` points into `repl` (the result of the block, of the
  hash, or of `rb_reg_regsub`) across `RESIZE_CAPA`.
- `string.c` `rb_str_each_codepoint`: `ptr` points into the frozen copy `rb_str_new4(str)`
  across `rb_yield`.
- `string.c` `rb_to_id`: `rb_raise` allocates its message before it reads
  `RSTRING_PTR(tmp)` of the result of `rb_inspect`; the guard is on the assignment.
- `string.c` `tr_trans` and `tr_setup_table` (`tr`, `tr!`, `tr_s`, `delete`, `squeeze`,
  `count`): the pointers into the strings of the arguments, which may be results of
  `to_str`, are read across `rb_hash_new` and `rb_hash_aset`.
- `sprintf.c` `rb_str_format`, signed conversions of a Bignum: `s` points into the result
  of `rb_big2str` across `CHECK` and `PUSH`, which resize the result; the string is stored
  in the `volatile` `tmp1`, as the unsigned branch next to it already does.

Judged safe:

- `bignum.c`: `bigzero_p`, `bigtrunc`, `bigfixize`, `get2comp`, `rb_big_pack`,
  `rb_quad_pack`, `big2ulong`, `big2ull`, `big2dbl`, `big_real_len`, `big_sparse_p`,
  `rb_big_cmp`, `rb_big_eq`, `rb_big_eql`, `rb_big_aref`, `rb_big_hash`, `rb_big_odd_p`,
  `rb_big_even_p`, `big_fdiv` and `big2str_orig` allocate nothing between the pointer and its
  last use (or take it after the allocation); `rb_big_clone`, `rb_uint2big`,
  `rb_ull2big`, `rb_big_unpack`, `rb_quad_unpack`, `dbl2big`, `rb_cstr_to_inum`, `bigsub`,
  `bigadd`, `bigmul1_single`, `bigmul1_normal`, `bigsqr_fast` and `big_lshift` take the
  pointers after the allocation; `rb_big_neg` and `big_rshift` use the object again after
  the allocation (`big_rshift` also holds its clone in a `volatile`); `rb_big2str0` uses the
  string afterwards and the Bignum only through its clone, which it resizes at the end;
  `big_split` reads `vds` across `bignew`, but both calls in `bigmul1_karatsuba` use `x` and
  `y` afterwards; `bigmul1_karatsuba` uses `t1` and `t2` again and holds its halves in
  variables whose address it passes; `rb_str_to_inum` holds its string through
  `StringValue` across `rb_cstr_to_inum`; `rb_big_and`, `rb_big_or` and `rb_big_xor` hold
  `x` and `y` in `volatile` variables.
- `marshal.c`: the `_dump` branch of `w_object` is patch 42. The other branches store the
  object in `arg->data`, which `mark_dump_arg` marks, before they read it; class paths
  (`w_class`, `w_unique`, `w_extended`, `class2path`) are held in the class's instance
  variables and the strings of `rb_id2str` by the symbol table; `r_object` holds its strings
  and Bignums in `volatile` variables or uses them afterwards; `marshal_load` holds a
  converted source through the address of `port` passed to `rb_scan_args`, and
  `id2encidx` holds its string through `StringValueCStr`.
- `time.c`: every function that takes `GetTimeval` of a Time it created returns or passes
  on that Time afterwards; the others read the receiver or an argument of a method; the
  wide values of `struct vtm` live on the stack; `time_strftime` holds its copy of the
  format through `StringValue`; `month_arg` and `utc_offset_arg` use the string again or
  do not allocate in between.
- `pack.c`: `pack_pack` uses `fmt` at the end (`OBJ_INFECT`) and holds each `from`
  through `StringValue`; `pack_unpack` reads the receiver and holds `fmt` through
  `StringValue`; `infected_str_new` copies before anything else allocates.
- `sprintf.c`: `%s` and `%p` read `RSTRING_PTR(str)` again after `CHECK`; `%c` uses `tmp`
  again and allocates nothing after the last read; the unsigned Bignum branch holds `tmp1`;
  `fmt` is held through `StringValue`; `rb_enc_vsprintf` and `rb_str_vcatf` use their
  result afterwards.
- `string.c`, other than the guarded places: the string is used after the allocation
  (`rb_str_conv_enc_opts`, `rb_str_succ`, `rb_str_inspect`, `str_gsub`, `rb_str_each_line`,
  the `awk` and regexp branches of `rb_str_split_m`, `rb_str_upto`, `rb_str_sum`,
  `rb_str_crypt`); the destination is allocated before any pointer is taken
  (`rb_str_plus`, `rb_str_times`, `rb_str_reverse`, `rb_str_dump`, `rb_str_justify`,
  `str_new4`, `str_replace_shared`); nothing allocates between the pointer and its last
  use (`casecmp`, `index`, `rindex`, comparison and hashing, `start_with?`, `end_with?`,
  the case conversions, `lstrip!`, `rstrip!`, the loops of `delete!`, `squeeze!` and
  `count`, `ord`); the pointer is only compared after the call (`p` in `rb_str_sub_bang`
  and `rb_str_scan`); the string is held elsewhere (`rb_id2name` and `sym_inspect` by the
  symbol table, the registers of a match by `$~`, the string branch of `rb_str_split_m`
  by `argv` or `$;`, `rb_str_format_m` by a `volatile`, `rb_str_intern` by its own guard);
  every caller in `string.c` of `rb_str_new5`, `str_new` and `str_buf_cat` uses the
  source afterwards.

## Verification

The flow has no aarch64 machine, so aarch64 is verified after the merge: the tests of
`tests/inner_pointers.rs` and the stressed suites run there in the CI job `gate-aarch64`
and on the owner's Jetson. On x86_64 the tests pass with and without the patches.

`docs/vendored-security.md` reviews the security fixes published after the release and lists
the defects that the fuzz targets and the sanitized run found; it does not list the guards
against the collector (patches 8 and 9 are not in it), so this change leaves it unchanged.

One edit outside the scope of the change: `xtask/src/gate.rs` names every test target of
`vkrgss-ruby192-sys` in its list `RUBY192_TESTS`, and its test fails for a target in no
part of the gate, so the new target `inner_pointers` is added there (two lines).

## Out of scope

Compiling with flags that change register allocation; Ruby 1.8.7.

## Follow-ups

- The audit covered the files that design.md names. The same pattern, a string passed as
  `RSTRING_PTR(...)` of a temporary to `rb_raise` or `rb_warn`, which allocate before they
  read it, also appears in `compile.c`, `error.c`, `variable.c`, `vm.c` and
  `vm_insnhelper.c`; an audit of the rest of the tree would be a change of its own.
- Found by the audit but not a matter of the collector, and left as they are: in
  `string.c`, `rb_string_value_cstr` returns a pointer taken before `rb_str_modify`
  (Ruby 1.9.3 takes it again), the `STR_ASSOC` path of `rb_str_cat` keeps a pointer into
  the string across `REALLOC_N`, and `rb_str_chomp_bang` keeps `p` across `to_str` of the
  separator, which may replace the receiver; in `pack.c`, `string.c` and `time.c`, `to_str`
  or `to_s` of an argument may change the format string or the array while it is read.
