# Fix the nightly fuzz findings

## Why

Issue #1 ("Nightly checks failed") is still open. Since the change
`make-the-nightly-checks-pass`, the audit passes; two fuzz jobs fail:

1. `fuzz (marshal)`, run 39227119222 (2026-10-19, at `07b24f80`): AddressSanitizer stack
   overflow in `get_min_match_length` (`third_party/oniguruma-5.9.6/regcomp.c:2035`) after
   681,271 inputs. `Marshal.load` builds a Regexp from a deeply nested pattern; stack
   exhaustion by nesting is set aside as a limit, but the list of recursive functions of the
   target `marshal` (`MARSHAL_RECURSIVE` in `crates/vkrgss-ruby-sys/tests/fuzz.rs`) holds only
   Marshal's own functions, not the regular expression engine's, so it counted as a finding.
2. `fuzz (parse)`, run 39110302699 (2026-10-18, at `353b52e5`): UndefinedBehaviorSanitizer at
   `third_party/ruby-1.8.7/gc.c:456:26`, "member access within misaligned address ... for type
   'RVALUE'" in `rb_newobj`, after 892,960 inputs: the head of the collector's free list points
   into the middle of an object. This is the defect that `make-the-nightly-checks-pass` could
   not reproduce (its design.md, "Follow-ups": a slot put on the free list twice by
   `rb_gc_force_recycle`, or while a node still points to it, suspected in `parse.y`). This
   time the workflow kept the input: 384 bytes of literals, heredocs (`<<EOS`, `<<-'RAW'`),
   `%w`/`%W` with interpolation and syntax errors, saved at
   `vkxp/target/fuzz-findings/nightly-39110302699/crash-dcde41704528cfbcab30022b43acac390eea3ca3`.

## What Changes

- The target `marshal` of both interpreters sets aside a stack overflow in the regular
  expression engine's recursive functions as well (the lists of `regex` reused).
- The free-list corruption of Ruby 1.8.7 is reproduced with the saved input (in a fresh
  process, after the corpus, or with collections forced at chosen allocations), its cause is
  found and fixed as a recorded patch, and the input is added under `fuzz/fixed/` with the test
  that replays fixed inputs. Ruby 1.9.2's parser is checked for the same defect.
- After the fix, the `parse` and `marshal` targets each run 30 minutes locally without a
  finding outside the listed limits.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the parser does not corrupt the collector's free list.

## Impact

The fuzz tests of `crates/vkrgss-ruby-sys` and `crates/vkrgss-ruby192-sys`,
`third_party/ruby-1.8.7` (and `ruby-1.9.2` if it has the defect) through recorded patches,
`fuzz/fixed/`, `docs/vendored-security.md` ("Found by the fuzz targets"), `CHANGELOG.md`.
