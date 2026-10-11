# Design

## Decisions

### A range that starts with a class

Decided by the owner's side after the first run, which found that the proposal had the
measurement backwards: the original runtime rejects `/[\d-z]+/` with "mismatch multibyte
code length in char-class range" (line 40 of `tests/data/regex.original.txt`), and the run
on the other machine accepted the pattern. Option 2 of the Follow-ups is taken.

In `next_state_val`, state `CCS_RANGE`: when the value before the hyphen is a class
(`*type == CCV_CLASS`), return `ONIGERR_MISMATCH_CODE_LENGTH_IN_CLASS_RANGE` before `*vs`
is read. Nothing else changes: a hyphen followed by the end of the class or by `&&` never
reaches that state and stays literal. `vs` is not given an initial value, for the reason
the first run recorded.

The test of the measured samples drops its adjustment of line 40, and the follow-up of the
change `embed-oniguruma` about that line is closed by this. The change is recorded as the
other local changes to vendored source are (the manifest and the description file of
`third_party/oniguruma-5.9.6`; if that directory has no patch list yet, one is started in
the form of `third_party/ruby-1.8.7/PATCHES.md`).

A test that does not depend on the compiler: `[\d-z]`, `[\w-%]`, `[\s-a]` and `[a\d-z]` are
errors with that message; `[\d-]`, `[\d-&&\d]` if the syntax accepts it today, and
`[a-z\d]` behave as before.

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

- Settled: the open question of the first run about `[\\d-z]` is decided above (the error of
  the original).
- The test of task 2.1 retries the whole of `run`, which starts the tool twice. "Text file
  busy" can only meet the first start, before anything is written, so the retry repeats
  nothing.
- Task 1.1 leaves two sentences that the patch makes stale, both outside the scope of this
  change: the follow-up about line 40 in the archived design of `embed-oniguruma`, and "a
  probe of 65 patterns gives the original's results except one error message" under "Ruby" in
  `docs/decisions.md`. The probe now gives the original's results without that exception.
