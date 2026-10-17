# Build the interpreters for Android

## Why

P3 of `docs/decisions.md` ("Platforms", Android). The feasibility measurement of
2026-10-16/17 on the owner's Pixel 7a (Android 17) and Galaxy S24+ (Android 16)
(`vkxp/target/p3-android/REPORT.md` of the workspace, experimental diff
`measurement-patches.diff` beside it) showed that the Vulkan backend passes all 88
differential tests on Mali-G710 and Xclipse 940 unchanged, and that both interpreters build
for `aarch64-linux-android` with small changes. At API 31, Ruby 1.8.7's `IO#eof?` answers
wrongly and `IO#readpartial` hangs, because bionic's `FILE` is opaque and `__freadahead`
exists only from API 34; with an API 34 build and a branch that uses it, its upstream
suites pass (670 tests, 0 failures). The owner decided on 2026-10-17: the minimum is API 34
(Android 14).

## What Changes

- Both interpreter crates build for `aarch64-linux-android` at API 34: configuration
  headers derived from aarch64 Linux without glibc's `FILE` internals and the functions
  bionic lacks (`eaccess`, `group_member`, `lchmod`; for 1.9.2 `backtrace` stays, API 33+),
  zlib from the NDK's sysroot, no `-lcrypt`, `-lpthread`, `-lrt`.
- Ruby 1.8.7's `io.c` reads the buffer of a `FILE` with `__freadahead` on Android, as a
  recorded patch.
- `String#crypt` raises `NotImplementedError` on Android, where bionic has no `crypt(3)`.
- `RUBY_PLATFORM` is `aarch64-linux-android`.
- `Math` raises the same errors as on Linux (the errno handling of the CORE-MATH shim covers
  the functions bionic's libm does not set `errno` for; if the macOS change has already
  added it, reuse it).
- A way to run the tests of the interpreter crates on a phone over adb
  (`cargo xtask android-test <crate>` or a script under `xtask`), with the data the tests
  need pushed and the paths taken from the environment instead of `env!` paths of the host.
- `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3` link for Android.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-platforms`: the interpreters build and pass their tests on Android (P3 not yet
  accepted).

## Impact

`crates/vkrgss-ruby-sys`, `crates/vkrgss-ruby192-sys` (headers, build scripts, signals,
tests), `third_party/ruby-1.8.7` and `third_party/ruby-1.9.2` (recorded patches only),
`xtask` (the adb test runner), `docs/building.md`, `docs/decisions.md` (API 34),
`CHANGELOG.md`. No change on Linux.
