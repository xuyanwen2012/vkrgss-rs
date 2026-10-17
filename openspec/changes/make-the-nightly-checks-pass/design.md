# Design

## Context

- `crates/vkrgss-ruby-sys/tests/fuzz.rs` (and its copy in `vkrgss-ruby192-sys`): `ended`
  maps exit status 0 to clean, the time-out and out-of-memory statuses to limits, anything
  else to a finding. The targets: `marshal` and `parse` with `max_len` 4096, `regex` 512.
- The failing lines of the nightly run are quoted in the proposal. The runner has 4
  threads and 16 GB; libFuzzer's memory limit is 2048 MB.
- No network in the flow: `cargo-deny` 0.20.2 cannot be installed there. Change the pin and
  `deny.toml` only as far as the release notes of 0.19 and 0.20 that you know require (field
  renames in `[advisories]` and `[licenses]`); if unsure, keep `deny.toml` and say in
  "Follow-ups" that the first nightly run checks it. `cargo xtask audit` needs `cargo-deny`
  installed locally; if this machine has an older one, do not upgrade it.

## Decisions

- **Stack overflow.** Read the report a process writes (it is already passed through): an
  AddressSanitizer `stack-overflow` whose first frames are in a function of a fixed list
  of recursive functions per target (for `regex`: the recursive functions of `regcomp.c`
  and `regparse.c` such as `get_min_match_length`, `parse_exp`, `parse_subexp`, `setup_tree`;
  for `marshal`: `r_object0` and its callers; for `parse`: the parser's recursive
  functions) counts as a limit named "stack" and is put aside with the input; any other
  stack overflow is a finding. Count them in the summary line as the other limits are.
- **Findings of the parsers.** Run `cargo xtask fuzz parse192` and `cargo xtask fuzz parse`
  with `--seconds` up to 3600 each (a corpus is kept between runs). For a finding: minimise
  the input, fix the defect with the smallest correct change as a recorded patch, add the
  input under `fuzz/fixed/` with the test that replays fixed inputs, and add a row to
  `docs/vendored-security.md` under "Found by the fuzz targets". For `eval.c:496`, check
  first whether the input defines something that makes `klass` a non-class (for example a
  method on an object whose singleton class is broken by a parse error) and whether a guard
  in `search_method` or a fix at the caller is right.
- **Artifacts.** In `nightly.yml`, a step `if: failure()` of the fuzz job uploads
  `target/tmp/fuzz/${{ matrix.target }}/artifacts/` with `actions/upload-artifact` pinned at
  `ea165f8d65b6e75b540449e92b4886f43607fa02` (v4, as `release.yml` has it), retention 30
  days.

## Out of scope

Raising libFuzzer's memory limit; fork mode; other targets.

## Follow-ups

- **`deny.toml` under `cargo-deny` 0.20.2.** The file already has the form that 0.18 asks for
  (`[advisories]` without the removed `vulnerability`, `notice` and `unsound` keys,
  `unmaintained` with a scope, `[licenses]` with `allow` only), and no change of 0.19 or 0.20
  that is known here renames a key it uses, so it is kept as it is. `cargo-deny` is not
  installed on the machine of this change and there is no network, so the first nightly
  run is what checks it.
- **The finding of `parse` at `eval.c:496` is not reproduced.** About two hours of `parse`
  (runs of 570, 690 and 3600 s, and 1500 s of a scratch build described below) did not
  report `eval.c:496` again. The runs found and this change fixes three other defects of the
  parsers (patches 40 to 43 of Ruby 1.8.7, 47 and 48 of Ruby 1.9.2, the last the nightly
  finding of `parse192`). One run of `parse` ended with a report of the same kind as the
  nightly one: UndefinedBehaviorSanitizer at `gc.c:456` (`rb_newobj`), "member access within
  misaligned address" of type `RVALUE`, where the head of the collector's free list pointed
  one byte into the name of the source file (`"fuzz"`, which `rb_source_filename` keeps one
  byte after the start of its buffer). So the free list was corrupted: a free slot had its
  second word written as a node's `nd_file`. One slot put on the free list twice by
  `rb_gc_force_recycle`, or put there while a node still pointed to it, would explain this
  and the nightly report alike, since a class whose slot is handed out a second time is no
  class any more. The input of that run (962 bytes of `while`, `begin`, `rescue`, `else`,
  `ensure` with syntax errors and bytes that are no valid UTF-8) is kept in
  `local/fuzz-findings/parse-freelist-c8720e17` of this clone. It does not fail on its own:
  not in a new process, not after the 50,248 inputs of the corpus in one process, not
  repeated 30 times with a collection forced every 1 to 150 allocations, and not with
  `GC.stress` (which rebuilds the free list at every allocation and so hides such a slot).
  A scratch build that aborts when `rb_gc_force_recycle` gets a slot that is already free
  (a temporary change of `gc.c`, not kept) did not trip in its 1500 s or on that input. The
  calls of `rb_gc_force_recycle` in `parse.y` (lines of `block_var`, the interpolation
  `#{}`, `dispose_string`, `heredoc_restore`, the end of a string term, `literal_concat`,
  `fixup_nodes`, `rb_parser_free` and the end of `yycompile`) were read without finding a
  path that recycles a live or a free slot. Left open: a guard in `search_method` would
  only hide a corrupted heap, so none is added; the next step is a run with that detector
  for longer, or one that also checks at each `rb_newobj` that the slot it hands out is
  free.
