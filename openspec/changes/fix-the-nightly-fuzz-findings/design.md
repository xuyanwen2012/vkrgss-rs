# Design

## Context

- Inputs (read only, copy into the clone): the crash input above, and
  `../../vkxp/target/fuzz-findings/parse-freelist-c8720e17` (962 bytes, the earlier report of
  the same kind from `make-the-nightly-checks-pass`, which did not reproduce on its own).
- The analysis of `make-the-nightly-checks-pass` (archived under
  `openspec/changes/archive/*make-the-nightly-checks-pass/design.md`, "Follow-ups"): calls of
  `rb_gc_force_recycle` in `parse.y` (block_var, the interpolation `#{}`, `dispose_string`,
  `heredoc_restore`, the end of a string term, `literal_concat`, ...) are the suspects; a scratch
  build that aborted on recycling an already-free slot did not trip in 1500 s.

## Decisions

- **Reproduce first.** Run the fuzz executable of the target `parse` on the input alone, then
  with `-runs` over the corpus and the input, and with the collector forced (`VKRGSS_GC_STRESS`
  or a debug knob that collects every N allocations, N over a range). Add instrumentation in a
  scratch build only: a check in `rb_gc_force_recycle` that the slot is live and not already on
  the free list (walk a bounded part of the list, or a side bitmap), and a poison of the
  recycled slot's second word so a later write through a stale pointer is caught. Once
  reproduced, bisect to the `rb_gc_force_recycle` call (or other cause) that frees an object
  still referenced.
- **Fix.** The smallest correct change: usually not recycling an object another node or
  string term still holds, as later Ruby 1.8 releases did (check the 1.8.7 branch history you
  know; there is no network). Recorded patch with the reason.
- **If it cannot be reproduced** in about two hours of trying: record what was tried and what
  was ruled out under "Follow-ups", keep the input under `fuzz/open/` (not `fixed/`), list it as
  a known open finding in `docs/vendored-security.md`, and do not tick task 2.1.
- **Ruby 1.9.2.** Its parser kept some `rb_gc_force_recycle` calls; check whether the same
  call exists there and whether the input (with 1.9 syntax adjustments) triggers anything in
  `parse192`.

## Out of scope

Rewriting the parser; other targets.

## Follow-ups

- **How the cause was found.** The saved input passed alone, 20 times in one process, and
  with a collection forced every 1 to 400 allocations. A scratch build (not kept: a changed
  `gc.c`, a hook in `parse.c` and a variable of the harness that forces collections) kept
  each recycled slot out of the free list until the next collection, marked with the line of
  `parse.y` that recycled it, reported a reference to such a slot found while marking, and
  aborted when a slot was recycled that was already free or recycled. After 712,736 inputs
  (300 s of `parse`) it aborted: `yycompile`, in the loop that recycles the varmaps of block
  variables made during the compilation, recycled a free slot. The input of that process
  failed only after others, which pointed at state left by error recovery. A `def` whose
  arguments have a syntax error, in a block, is the case: `local_push(0)` of the `def` has
  run, error recovery discards the rule, and its table stays on top. The varmap with which
  `local_push(0)` cuts the chain of block variables is the only thing that keeps the saved
  chain (the varmap of the block) from the collector, and it is dropped when the block ends.
  The `local_pop` of the next enclosing rule, at the latest that of the script, then pops
  the `def`'s table and makes the saved chain current again, after the collector may have
  freed it and handed its slots out again, and `yycompile` recycles every slot along it. In
  the scratch build `[1].each { def m(&b,) 1 }` followed by a statement made that loop recycle
  a live node; in the unchanged sanitized target it ends with the nightly report,
  UndefinedBehaviorSanitizer at `gc.c:456:26` in `rb_newobj`, misaligned `RVALUE`: in a fresh
  process when two array literals of 1,500 elements each follow (`local/inputs/t2.rb` of
  this clone, 20 KiB), and at the fifth compilation in one process for the 2.5 KiB input kept as
  `fuzz/fixed/parse/def-arguments-error-in-block.rb`. Without the fix, the test that replays
  the fixed inputs (20 compilations of it) ends with a segmentation fault.
- **The fix** (patches 46 and 47) keeps the chain saved by each `local_push(0)` marked until
  the end of the compilation, in a list beside `deferred_nodes`, so that a popped table
  restores live varmaps; no table is popped otherwise than before. With it, the scratch build
  above ran 4,300,694 inputs of `parse` in 900 s without a recycled free slot, without a
  slot recycled twice and without a write to a recycled slot. Its remaining reports of
  references to recycled slots come from objects that only the conservative scan of the
  stack reaches (varmaps and the new list's nodes from earlier compilations, garbage), and
  from `lex_strterm` between `heredoc_restore` and the allocation of the next string term;
  the unchanged collector does not mark a free slot, so both are harmless.
- **The saved inputs themselves** (`nightly-39110302699.rb`, kept beside the reproducer, and
  `parse-freelist-c8720e17`) pass with and without the fix: alone, and after the 60,792 inputs
  of the local corpus in one process. For the latter a scratch driver linked the target with
  the sanitized libraries and replayed a list of files in order, since libFuzzer orders a
  corpus by size; built with the unfixed parser, the same driver reports the reproducer at
  `gc.c:456`. The nightly input has no `def`, `class` or block. Without the fix `yycompile`
  follows the restored chain through whatever the freed slots then hold and recycles the
  objects along it, which can outlive the compilation, so a corruption made by an earlier
  input of a process shows in a later one; the inputs before it in the nightly process were
  not saved, so this is the explanation, not a demonstration, for this input and for the
  report at `eval.c:496` of `make-the-nightly-checks-pass` (a class whose slot was handed out
  again).
- **Ruby 1.9.2** has no such chain and no such call: its local tables and block variables
  are `vtable`s in memory of its own, `dyna_pop` pops down to the table it saved, and its
  `yycompile` recycles nothing. Its calls of `rb_gc_force_recycle` are those of string terms,
  `heredoc_restore`, `dispose_string`, `literal_concat` and the interpolation, for which the
  scratch build of 1.8.7 reported no reference to a recycled slot, and the joining of the
  pieces of a regexp literal, which 1.8.7 does not have (its fuzz finding is the input
  `regexp-pieces-after-interpolation.rb` of `vkrgss-ruby192-sys`). The two saved inputs, the
  reproducer and the 20 KiB variant ran through the target `parse192` (built with
  `cargo xtask fuzz parse192`, which ran 49,612 inputs in 6 s) without a report.
- **The runs of task 2.1**, with the fix, on this machine (12 threads; the two at once):
  `cargo xtask fuzz parse --seconds 1800` ran 6,536,584 inputs without a finding (22 inputs
  exhausted the stack by nesting, 17 processes ran into the memory limit);
  `cargo xtask fuzz marshal --seconds 1800` ran 17,940,369 inputs without a finding, and its
  one exhausted stack was the nightly one of `marshal`, `get_min_match_length` at
  `regcomp.c:2035`, now set aside by task 1.1. The sanitized run of `vkrgss-ruby-sys`
  (`upstream_suites_pass_under_the_sanitizers`) passes.
- **Left as it is.** A table that error recovery leaves on top still takes the locals of the
  enclosing code until it is popped, and the tables are not freed (the memory limit of the
  target already sets aside the tables of scopes that a syntax error leaves open); a script
  with that error is refused, so only memory and not behaviour depends on it.
  `docs/development.md` names the engine's functions as set aside "for a pattern and for a
  script"; since task 1.1 they are also set aside for `Marshal.load`, which the sentence does
  not say. The file is outside the scope of this change.
