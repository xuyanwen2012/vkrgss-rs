# Design

## Decisions

### The hyphen after a class

In `parse_char_class`, case `TK_CC_RANGE`, state `CCS_VALUE`: when the value before the
hyphen is a class (`val_type == CCV_CLASS`), the hyphen is taken as the literal character
`-` (the path of `range_end_val` / a single-byte value), with the warning the neighbouring
cases give if warnings are on, instead of entering `CCS_RANGE`. `[\d-z]` is then `\d`, `-`
and `z`. This is what Oniguruma 6 does ("[\w-%]" in its sources). The measured sample
(`"1-z"` matched whole, `gsub` and `scan` as recorded in
`tests/data/regex.original.txt`) holds with it.

`vs` is also initialised to 0 where it is declared, so that no path can read an
indeterminate value.

The change is recorded as the other local changes to vendored source are (the manifest and
the description file of `third_party/oniguruma-5.9.6`; if that directory has no patch list
yet, one is started in the form of `third_party/ruby-1.8.7/PATCHES.md`).

A test that does not depend on the compiler: the patterns `[\d-z]`, `[\w-%]`, `[\s-a]`,
`[a\d-z]` and `[\d-]` are evaluated and their matches asserted (digits, the hyphen and the
letter match; a character between them in code order, such as `A` for `[\d-z]`, does not).
Before the patch at least the "does not match `A`" part fails or the pattern is an error,
whichever value the stack holds.

### The fake tool of the audit tests

`fake_cargo` in `xtask/src/audit.rs` keeps writing a script, and the start of it is made
safe against "text file busy": the tests of that module that execute a written file take a
module-wide lock for the write and the runs (`static Mutex`), and the helper that runs the
tool in tests retries a start that fails with `ETXTBSY` (raw OS error 26) a bounded number
of times with a short sleep, because a process forked by another test binary's thread
cannot be excluded by a lock inside this module. Only test code changes; `run` itself is
not given a retry.

### Warnings for uninitialised variables

Once, by hand: the C files that the build scripts compile for the interpreter and for
Oniguruma are compiled with `-O2 -Wuninitialized -Wmaybe-uninitialized` with gcc, and with
`-Wsometimes-uninitialized -Wconditional-uninitialized` with clang (a separate
`CARGO_TARGET_DIR`; the flags through `CFLAGS`). Each report is read and listed under
"Found" as real (then fixed with a recorded patch, like the one above) or as a false
positive with one line of reason. The flags are not added to the build.

## Found

Compiled on the development machine with gcc 16.2.1 and clang 22.1.8. The build script passes
`-w`, which silences every warning whatever `CFLAGS` adds, so the flags through `CFLAGS` and a
separate `CARGO_TARGET_DIR` reported nothing. Each file of `CORE_SOURCES`, `ext/zlib/zlib.c`
and each file of `ONIGURUMA_SOURCES` was therefore compiled by hand with the command line of
the build script (its flags, defines and include paths) without `-w` and with the flags above.
gcc gave two reports and clang six; one is in both.

- `time.c:2034`, `gmt` in `time_mload` (gcc `-Wmaybe-uninitialized`, clang
  `-Wsometimes-uninitialized`): real. For the older of the two marshalled formats `gmt` was
  never set and its indeterminate value was stored in the object. Fixed: patch 26 of
  `third_party/ruby-1.8.7/PATCHES.md` sets it to 0.
- `eval.c:10857`, `*sp` in `stack_extend` (gcc): false positive. The read of the memory that
  `alloca` just returned is deliberate, to touch the grown stack; the value goes into padding
  that nothing reads.
- `array.c:2449`, `item` in `rb_ary_fill` (clang): false positive. It is read only without a
  block, and without a block `rb_scan_args` with `"12"` sets it or raises.
- `string.c:2024`, `repl` in `rb_str_sub_bang` (clang): false positive. It is read only when
  `iter` is 0, which means `argc == 2`, where it is set; any other count raises.
- `string.c:2149`, `repl` in `str_gsub` (clang): false positive, for the same reason.
- `string.c:2193`, `match` in `str_gsub` (clang): false positive. The function returns before
  the loop when the first search fails, so the loop that sets `match` runs at least once.
- `regparse.c:4416`, `val_type` in `parse_char_class` (clang): false positive. It is read only
  in the state `CCS_VALUE`, which only `next_state_val` and `next_state_class` set, and both
  set the type with it.

Neither compiler reports `vs` of `parse_char_class`, the variable of task 1: its address is
passed to `next_state_val`, which ends this analysis in both.

## Follow-ups

- **Open question that stops task 1.1: what the original does with `[\d-z]`.** The proposal and
  the decision above say that the original runtime matches `"1-z"` with `/[\d-z]+/` and that
  the measured file records the match. It records the opposite. Line 40 of
  `crates/vkxp-ruby-sys/tests/data/regex.original.txt` is

  `"/[\\d-z]+/" "1-z" => SyntaxError: (eval):1: mismatch multibyte code length in char-class range: /[\d-z]+/`

  and the test `results_equal_those_measured_on_the_original` has, since the change
  `embed-oniguruma`, expected a `SyntaxError` there with only the message adjusted ("empty
  range in char class"; see the Follow-ups of that change). The source agrees: in
  `next_state_val` of 5.9.6 the branch for a range whose ends have different types still
  holds, inside `#if 0`, `return ONIGERR_MISMATCH_CODE_LENGTH_IN_CLASS_RANGE`, the message
  the original gives. The original's older engine rejects a range that starts with a class;
  5.9.6 removed the check and reads the uninitialised `vs` instead.

  On the development machine gcc 16.2.1 and clang 22.1.8 both give the `SyntaxError` "empty
  range in char class", so the test passes here; the run that failed elsewhere must have had
  a small value in `vs` and accepted the pattern as a range, the reverse of what the proposal
  describes.

  So the requirement "the hyphen is the literal `-`" and its first scenario cannot hold
  together with the scenario "Measured sample: every result equals the measured one", nor
  with the scenario of the main spec "a pattern that is an error on the original is a
  `SyntaxError` with the same message". The owner has to choose:

  1. The literal hyphen, as the requirement says and as Oniguruma 6 does. The pattern is then
     accepted where the original rejects it; the test of the measured samples has to replace
     the whole of line 40, and the scenario "Measured sample" and the one of the main spec
     need an exception for it.
  2. The error of the original: in the state `CCS_RANGE`, a range that starts with a class is
     `ONIGERR_MISMATCH_CODE_LENGTH_IN_CLASS_RANGE` (restoring the check for
     `*type == CCV_CLASS`). Every compiler then gives the original's result with the
     original's message, the adjustment of line 40 in the test goes away, and the follow-up
     of `embed-oniguruma` about that line is closed. The requirement and the first scenario
     of this change's spec have to be rewritten for it.

  Either way `vs` is never read uninitialised. Initialising `vs` to 0 alone was not done: it
  would make `[\d-z]` the range from NUL to `z` on every compiler, which is neither. Nothing
  of task 1.1 is in the tree; task 4.1 stays unticked with it.
- The test of task 2.1 retries the whole of `run`, which starts the tool twice. "Text file
  busy" can only meet the first start, before anything is written, so the retry repeats
  nothing.
