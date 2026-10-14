## Context

`crates/vkxp-ruby192-sys/build.rs` compiles the vendored source with `cc`, which takes
`CFLAGS` of the environment. Reproduced on the development machine on 2026-10-13:

```sh
CARGO_TARGET_DIR=$PWD/target/fortify CFLAGS='-O2 -D_FORTIFY_SOURCE=3' \
    cargo test -p vkxp-ruby192-sys --locked --test upstream
```

gives "3 file(s) of test/ruby did not pass" with `ruby/test_dir.rb` (after six tests),
`ruby/test_require.rb` (after four) and `ruby/test_system.rb` (after one), each ending with
`*** buffer overflow detected ***: terminated`. On the runner the stressed run
(`quick_files_pass_under_collector_stress`) fails as well. Which call it is has not been
looked for; the three files have reading the working directory, expanding paths and starting
a process in common.

## Goals / Non-Goals

Goals: the interpreter does not end in a check of the fortified C library; the gate passes
on Ubuntu 24.04 as on the development machine.

Non-goals: turning the check off (`-U_FORTIFY_SOURCE`, `-D_FORTIFY_SOURCE=0`) in the build
script; sanitizer runs of 1.9.2 (a follow-up of `vendor-ruby-1-9-2`); anything of the
binding.

## Decisions

- The source is corrected, the check is not silenced. A place where the check is right is
  a write past a buffer and gets the fix of the later releases of Ruby where there is one
  (named in the patch log), or one of our own. A place where the check misreads an idiom
  (a structure that ends in an array of one element used as longer, a pointer into a larger
  allocation) is rewritten so that the compiler knows the true size; the patch log says
  which of the two each is.
- If, against expectation, a place can be shown to be correct and cannot be written so that
  the check accepts it, the build script may switch the check off for that one file, with
  the reason in the patch log and under "Follow-ups" here. Not for the whole library.
- The fortified build is not a new step of the gate: the CI runner's compiler does it in
  every run. `docs/development.md` gets the command above for trying it on a machine whose
  compiler does not.
- Ruby 1.8.7 is tried with the same flags (`cargo test -p vkxp-ruby-sys --locked` in a
  target directory of its own). It passes on the runner, so nothing is expected; a finding
  is recorded and, if it is a check of the same kind, fixed the same way.

## Implementation notes

- Finding the place: run the failing test file alone in the fortified build under `gdb`
  (`break __chk_fail`, `bt`) or with `ulimit -c` and a core; the harness of the suites names
  the executable and its arguments. The process ends with SIGABRT.
- A corrected vendored file changes its hash: the manifest and the patch chain of
  `PATCHES.md` follow as for the other patches (the test of the manifest shows how).
- Use a target directory of its own (`CARGO_TARGET_DIR=$PWD/target/fortify`) so that the
  plain build is not rebuilt back and forth.
