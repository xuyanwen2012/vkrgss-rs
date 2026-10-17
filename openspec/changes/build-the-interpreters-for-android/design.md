# Design

## Context

Material (read only): `../../vkxp/target/p3-android/` — `REPORT.md`, `measurement-patches.diff`
(the scratch changes: headers, build scripts, crypt stub, ITIMER_VIRTUAL, io.c), `env34.sh`
(the cross environment: NDK r30 at `/home/doremy/android-ndk-r30`, linker
`aarch64-linux-android34-clang`), `push/` (the scripts that ran tests on the phones) and
`logs/`. Phones on USB of this machine, free to use: Pixel 7a `3A021JEHN02756`, Galaxy S24+
`R5CY21Y3VEV`; use only `adb -s <serial> shell` and `adb push` into
`/data/local/tmp/vkrgss-test/`, and remove that directory after a run.

## Decisions

- **Headers.** Start from the measured ones; keep `HAVE_BACKTRACE` for 1.9.2 (API 34 > 33).
  A test compares each Android header with its aarch64 Linux counterpart and allows only the
  listed differences, as for aarch64 Linux.
- **io.c.** The branch of the measurement (`__ANDROID__ && __ANDROID_API__ >= 34`, using
  `<stdio_ext.h>` and `__freadahead`) as a recorded patch; the build fails with a clear error
  below API 34.
- **crypt.** No stub that returns NULL: `String#crypt` raises `NotImplementedError` with a
  message that names Android, through a small change in `string.c` under `__ANDROID__` (a
  recorded patch) or a `crypt` of the compat layer that sets `errno = ENOSYS` and lets the
  caller raise; choose the smaller. A DES implementation is out of scope.
- **ITIMER_VIRTUAL.** A local constant where the `libc` crate lacks it for Android.
- **zlib.** `libz-sys` links the system `libz.so` on Android and exports no include path;
  take the headers of the NDK sysroot. Note this in `docs/building.md`.
- **adb runner.** A command of `xtask` that cross-builds the test binaries of a crate for
  Android, pushes them with the files they read (the vendored test trees for the upstream
  suites, the test data), runs them on a chosen device with `--test-threads=1` under
  `umask 022`, collects the result and removes the directory. Tests that read `env!` paths
  of the host take them from an environment variable when it is set. Tests that need a host
  C compiler or the host's file system layout are marked as not run on Android, with the
  reason. Hard links are refused by SELinux in `/data/local/tmp`: the upstream allow-list
  gets those cases for Android only.
- **If adb is not reachable from the flow**, implement and cross-build, and say in
  `design.md` that the device runs are done at the merge.

## Out of scope

Presentation (`VK_KHR_android_surface`), the APK, the seccomp filter of the guest, starting
the guest from an app.

## As implemented

- **API level.** Both build scripts read `__ANDROID_API__` of the C compiler (one
  preprocessed probe) and stop below 34 with a message naming it; the `io.c` patch also has an
  `#error` below 34. Verified with `aarch64-linux-android31-clang` for both crates.
- **crypt.** The `string.c` patch of each interpreter (1.8.7 patch 45, 1.9.2 patch 49): no
  `crypt` symbol is needed, so no compat file. The headers drop `HAVE_CRYPT` and
  `HAVE_LIBCRYPT` with the others bionic lacks.
- **errno.** `third_party/core-math/vkrgss/vkrgss_math.c` sets `EDOM` for a NaN from non-NaN
  arguments and `ERANGE` for an infinity from finite ones, under `__ANDROID__` only; its
  `README.vkrgss.md` says so (a few lines of documentation beside the shim).
- **Paths.** `tests/common/mod.rs` of each crate has `host_dir(name, built)`: the runtime
  variable of the same name wins over the `env!` value. `config_headers.rs` and `targets.rs`
  of Ruby 1.9.2 read `CARGO_MANIFEST_DIR` the same way inline, because they build on targets
  where the crate is empty. `build_ruby_executable` returns `VKRGSS_RUBY_EXECUTABLE` or
  `VKRGSS_RUBY192_EXECUTABLE` when set, so the suites, `symbols.rs`, `hashes.rs`,
  `symbol_ids.rs` and the signal and path variants run on the phone with the executable
  that `cargo xtask android-test` linked on the host.
- **Not run on Android.** The two `layout.rs` tests of Ruby 1.9.2 and the one of 1.8.7, which
  compile a C program at run time (`ignore` with that reason); the instrumented tests are
  opt-in everywhere.
- **Allow lists.** Ruby 1.8.7: `LEFT_OUT` in `tests/upstream.rs` (`test_file_exhaustive.rb`,
  68 tests, hard links). Ruby 1.9.2: `tests/upstream.android.allow`, added to
  `upstream.allow` on Android: `test_file_exhaustive.rb` (hard links), `test_crypt`, and
  `test_segv_test`, because bionic's linker installs debuggerd's handler for SIGSEGV and
  `install_sighandler` of `signal.c` keeps a handler it finds.
- **Device runs, 2026-10-17.** `cargo xtask android-test` for both crates on the Pixel 7a
  (`3A021JEHN02756`, SDK 37) and the Galaxy S24+ (`R5CY21Y3VEV`, SDK 36): all 29 test
  binaries of `vkrgss-ruby-sys` and all 38 of `vkrgss-ruby192-sys` pass on each phone,
  `upstream_stress` included; `/data/local/tmp/vkrgss-test` is gone afterwards.

## Follow-ups

- The layout of the FFI structures on bionic is not compared with the C headers on a phone:
  `layout.rs` would need its C program cross-compiled on the host and pushed.
- The executable that `cargo xtask android-test` links repeats `build_ruby_executable` of
  both crates (the `Init_ext` and the extensions); a test of `xtask` compares the list of
  extensions, not the whole text.
- If the host side of `cargo xtask android-test` is killed, the processes on the phone and
  `/data/local/tmp/vkrgss-test` stay until the next run removes the directory.
- On Android a crash of the interpreter ends in debuggerd's handler, not in Ruby 1.9.2's
  `[BUG]` report; whether the guest wants Ruby's handler there is open.
