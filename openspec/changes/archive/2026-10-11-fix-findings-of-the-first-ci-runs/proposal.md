# Fix what the first runs on another machine found

## Why

The gate was run for the first time on a GitHub runner and in a container of Ubuntu 24.04
(gcc 13.3). Two findings, neither of which shows on the development machine:

- `results_equal_those_measured_on_the_original` (`crates/vkxp-ruby-sys/tests/regex.rs`):
  `/[\d-z]+/` matches `"1-z"` there, where the original runtime raises a `SyntaxError` and so
  does the build of the development machine. In `parse_char_class` of
  Oniguruma 5.9.6 a class such as `\d` leaves the state `CCS_VALUE` with the type
  `CCV_CLASS` and never sets `vs`; a following `-` makes the state `CCS_RANGE`, and the next
  character reaches `next_state_val`, which reads `*vs` as the start of the range. `vs` is
  uninitialised, so the result depends on what the compiler left on the stack.
- `audit::tests::the_check_holds_the_lock_file_and_its_status_is_the_outcome` (`xtask`):
  `could not start /tmp/.tmpXXXX/cargo`. The test writes a shell script and executes it
  while other tests of the binary start processes of their own; a process forked between
  the write and the close still holds the file open for writing, and `exec` fails with
  "text file busy". A known hazard of tests that write and execute a file in a
  multi-threaded test binary.

## What Changes

- A range that starts with a class is the error the original runtime gives, with its
  message; no uninitialised value is read.
- The audit tests start their fake tool in a way that cannot meet a file still open for
  writing.
- The C files of the interpreter and of Oniguruma that the build compiles are compiled once
  with the warnings for uninitialised variables, and what they report is judged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: a range that starts with a class in a character class.

## Impact

`third_party/oniguruma-5.9.6/regparse.c` with a recorded patch, tests of `vkxp-ruby-sys`,
`xtask/src/audit.rs` (tests only).
