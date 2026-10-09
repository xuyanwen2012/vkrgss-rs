# Tasks

## 1. Vendored source

- [x] 1.1 Verify `local/ruby/ruby-1.8.7-p374.tar.gz` against the SHA-256 in `specs/embedded-ruby/spec.md` (if the file is absent, download it from `https://cache.ruby-lang.org/pub/ruby/1.8/ruby-1.8.7-p374.tar.gz` into `local/ruby/` first) and copy the file set listed in `design.md` into `third_party/ruby-1.8.7/` at its original relative paths. Verify: `find third_party/ruby-1.8.7 -type f | wc -l` is between 155 and 170 and no file outside the listed set is present.
- [x] 1.2 Add `MANIFEST.sha256` (upstream hash per vendored path), `PATCHES.md`, a `patches/` directory and `README.vkxp.md` (origin, licence, how the subset was chosen) to `third_party/ruby-1.8.7/`, and a third-party notice in the repository `README.md`. Verify: `sha256sum -c` over the manifest passes from inside `third_party/ruby-1.8.7/`.
- [x] 1.3 Add the manifest test to `vkxp-ruby-sys`. Verify: `cargo test -p vkxp-ruby-sys manifest` passes; changing one byte of a vendored file makes it fail naming that file (revert afterwards).

## 2. Build script

- [x] 2.1 Check in `local/ruby/config.h` as `crates/vkxp-ruby-sys/config/x86_64-unknown-linux-gnu/config.h` and write `build.rs` as described in `design.md` (fixed file list, pinned flags, hidden visibility, clang extras, static zlib through `libz-sys`, link libraries, rerun rules, error for other targets). Verify: `cargo build -p vkxp-ruby-sys` succeeds with gcc, and `CC=clang cargo build -p vkxp-ruby-sys` succeeds when clang is installed.
- [x] 2.2 Apply the `crc_table` patch to `ext/zlib/zlib.c`, store it under `patches/` and record it in `PATCHES.md` with the patched file's hash. Verify: the manifest test of 1.3 still passes and the compiler no longer warns about that line.
- [x] 2.3 Document build prerequisites (C compiler, libcrypt development files) in `README.md`. Verify: the README's prerequisite table lists both.

## 3. FFI

- [x] 3.1 Write the `ffi` module: types, constants, struct layouts, fixnum and type helpers, class and exception globals, and the function declarations named in `design.md`, including the two functions that no header declares. Verify: `cargo build -p vkxp-ruby-sys` has no warnings and every `unsafe` block carries a safety comment.
- [x] 3.2 Add the layout test that compiles a C program against the vendored headers and compares `sizeof` and `offsetof` of the mirrored structs and the values of the special constants with the Rust side. Verify: `cargo test -p vkxp-ruby-sys layout` passes; temporarily swapping two fields of a Rust struct makes it fail.

## 4. Interpreter thread

- [x] 4.1 Implement `Interpreter::start`, `run` and shutdown as described in `design.md` (16 MiB stack, stack limit with 1 MiB headroom, `Zlib` registration, synchronous output, single-interpreter guard). Verify: tests for the start-up-state scenarios and for "work from another thread" and "second interpreter".
- [x] 4.2 Save and restore signal dispositions around `ruby_init` and at shutdown. Verify: tests for the two signal scenarios, comparing `sigaction` results for every listed signal before and after.
- [x] 4.3 Add the recursion tests. Verify: the three unbounded-recursion patterns each raise `SystemStackError` and the interpreter answers `1 + 1` afterwards, on the default stack size and on 32 MiB.

## 5. Calling in and out

- [ ] 5.1 Implement `eval_protected(source, file, line)` returning the value or class name, message and backtrace. Verify: tests for the three protected-evaluation scenarios and the interpreter-identity scenarios.
- [ ] 5.2 Implement the helpers for defining classes, modules, methods and global functions backed by host functions, with the panic and raise discipline of `design.md`. Verify: tests for the three host-function scenarios.
- [ ] 5.3 Expose rooting of values and test it. Verify: the rooted-string scenario passes with collections forced in a loop.
- [ ] 5.4 Add the Zlib tests. Verify: both `crc_table` scenarios and a deflate/inflate round trip pass.

## 6. Ruby's own suites

- [ ] 6.1 Add the integration test that builds the test executable (`main.c`, `fcntl`, `stringio`, `Init_ext`) against the archives from the build script. Verify: the executable prints `ruby 1.8.7 (2013-06-27 patchlevel 374) [x86_64-linux]` for `-v`.
- [ ] 6.2 Run `test/ruby` (one process per file, several at a time), `test/zlib` and `sample/test.rb` from that test and assert the totals. Verify: `cargo test -p vkxp-ruby-sys upstream` reports 625 core tests, no failure and no error, in under 60 seconds.

## 7. Symbols and documentation

- [ ] 7.1 Add the symbol test: list the dynamic symbols of a test executable that links the interpreter and assert the names of the spec are absent. Verify: the test passes, and fails if `-fvisibility=hidden` is removed from the build script (restore afterwards).
- [ ] 7.2 Write the crate documentation: the threading rule, the two rules for entering and leaving Ruby, rooting, and the SIGVTALRM note. Verify: `cargo doc -p vkxp-ruby-sys --no-deps` builds without warnings.

## 8. Integration check

- [ ] 8.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate embed-ruby-interpreter --strict` passes.
