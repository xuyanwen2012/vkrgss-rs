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

None yet.
