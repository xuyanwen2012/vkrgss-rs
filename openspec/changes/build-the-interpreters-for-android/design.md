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

## Follow-ups

None yet.
