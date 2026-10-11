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

To be filled in.

## Follow-ups

None yet.
