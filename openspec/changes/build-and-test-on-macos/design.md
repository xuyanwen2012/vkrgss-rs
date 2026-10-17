# Design

## Context

Read first `../../vkxp/target/p2-macos/REPORT.md` (the measurement), and use as material
`scratch-all-experiments.diff`, `patch1-portability.diff`, `patch3-ruby-darwin-target.diff`
and `headers/` in the same directory (read only). The headers to start from are
`headers/ruby187-aarch64-darwin-noerror.config.h` and
`headers/ruby192-aarch64-darwin-noerror.config.h` (configure run with Apple clang's
implicit-declaration errors downgraded, so `NEGATIVE_TIME_T`, `HAVE_VA_ARGS_MACRO` and
`LOCALTIME_OVERFLOW_PROBLEM` come out as on Linux); Oniguruma's header is that of
aarch64 Linux.

## Decisions

- **Where the work runs.** If this change runs on the Mac (flow `@user/gpu_campaign` with the
  box on the Mac), build and test there with `DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib
  VK_LOADER_DRIVERS_SELECT='*MoltenVK*' VKRGSS_TEST_DEVICE='Apple M4'`, and run the Linux
  gate too only if a Linux machine is reachable; otherwise the Linux gate is run at the
  merge by the owner's session. If it runs on Linux, every macOS path is behind `cfg` and
  verified at the merge on the Mac.
- **Headers.** Copy the measured headers to `config/aarch64-apple-darwin/`, set
  `RUBY_PLATFORM` to `aarch64-darwin` and the 1.9.2 hand edits (`RUBY_EXEC_PREFIX`, no
  `STACK_END_ADDRESS`) as on Linux, with a comment naming where they come from. A test like
  the one for aarch64 Linux checks the platform string.
- **Errno of `Math`.** In the shim of CORE-MATH, for the functions where C99 asks for a pole
  or range error (`log(0)`, `atanh(±1)`, overflow of `exp`, `pow`, `cosh`, `sinh`, `tgamma`,
  `lgamma` at poles), set `errno` to `ERANGE` itself when the result is infinite from finite
  arguments, on every platform; on Linux this matches what glibc does, so nothing changes
  there. A test on Linux checks `errno` after those calls through the shim.
- **Parent death.** The guest gets an abstraction (`parent_watch` or similar) with the
  Linux implementation unchanged and a macOS one with kqueue; the copier and the signal
  setup use it.
- **Headless default device.** A function `default_device()`; Linux keeps `llvmpipe`; macOS
  takes the first device whose name starts with `Apple`.
- **Tests depending on Linux.** `cfg(target_os = "linux")` with a line of reason for `/proc`,
  inotify and ELF tests; the case-sensitivity tests of `vkrgss-vfs` and `xtask` detect a
  case-insensitive temporary directory and assert the behaviour of such a file system
  instead of skipping; the RGSS3 test with a file name that is not UTF-8 is kept to Linux
  with a follow-up (games that write Shift-JIS names on APFS); `cargo xtask dist` stays
  Linux-only and says so on macOS.
- **No CI for macOS** (decision of 2026-10-15). `docs/building.md` gets a section "macOS"
  with the commands above.

## Edits outside the impact list

- `crates/vkrgss/src/process.rs`: macOS refuses to set the read timeout of a Unix socket
  (`EINVAL`) whose other end is closed already, which a guest process with a very short game
  had done before the program accepted it; the run then failed with "Invalid argument". On
  macOS that error of `set_read_timeout` is now ignored in `attached`: what the guest sent
  is still read, and no read blocks on a closed connection. A few lines, needed for the
  guest-process tests of `vkrgss` to pass on the Mac.

## Verification on the Mac

The change ran on the owner's MacBook (Apple M4, macOS 27, MoltenVK), with no Linux machine
at hand. Task 5.1 was checked there in place of the Linux gate: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked` with the environment above (3448 tests passed, none
failed), and `openspec validate build-and-test-on-macos --strict`. The clippy and test runs
were repeated at `a874d44` through `~/vkrgss-flows/tmp/evidence/check.sh`, which writes the
command, the commit, the state of the tree, the environment, the times and the exit status
beside the output into `clippy.log` and `test.log` of that directory: both exited 0, the
tests with 3448 passed, 0 failed and 11 ignored. The Linux gate
(`VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate`) is run by the owner's session at the
merge.

## Out of scope

`.app` packaging, loading libvulkan and MoltenVK from the bundle, signing, a windowed run,
audio output, the real corpus, `sandbox_init`.

## Follow-ups

- **`IO#pos` after `IO#readpartial` in Ruby 1.8.7 on macOS.** BSD stdio answers `ftello`
  from the offset it last knew and does not see the read that `readpartial` makes on the
  descriptor, so `pos` is 0 instead of 2 in `test_io.rb#test_readpartial_pos`, which is kept
  to Linux in `tests/upstream.rs` of `vkrgss-ruby-sys`. Whether a game can meet it, and
  whether a recorded patch of `io.c` should make the position right, is open.
- **File names that are not UTF-8 on macOS.** The file system of macOS refuses a name that
  is not valid UTF-8, so a game of VX Ace that writes a file under a name in Shift_JIS (bytes
  labelled UTF-8 that are not) fails where it works on Linux and on the original. The test
  `names_labelled_utf_8_and_names_of_other_characters_are_files` of `vkrgss-binding-rgss3` is
  kept to Linux. Whether such names should be mapped to valid ones on macOS, and how they
  would be found again, is open.
- **The crash report of Ruby 1.9.2 on macOS.** `test_rubyoptions.rb#test_segv_test` is kept
  to Linux: the C backtrace has the format of `backtrace_symbols` of macOS, which the
  pattern of the test does not match. Only the text of a crash report differs.
- **`test_thread.rb#test_local_barrier` of Ruby 1.9.2.** It failed once in a run of the whole
  workspace on the Mac, its child printing `7: cont` after `exit.`, which the test's own
  threads may do; it passed in every other run. At `a874d44`, 220 runs of its script alone
  with the test executable of `vkrgss-ruby192-sys` (40 one after another, then 180 in twelve
  parallel workers of 15) all exited with status 0 and ended with `exit.`; the last line of
  each run is kept in `~/vkrgss-flows/tmp/evidence/lb-*.log` on the Mac, outside the
  repository. Left in the suite; whether it should be listed if it recurs is open.
