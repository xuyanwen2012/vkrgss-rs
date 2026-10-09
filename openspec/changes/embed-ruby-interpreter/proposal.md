# Proposal

## Why

Game scripts are Ruby 1.8 programs, and `docs/decisions.md` settles that vkxp runs them on a
vendored Ruby 1.8.7-p374. `vkxp-ruby-sys` is still an empty stub, so nothing can evaluate a line
of Ruby, and the Ruby binding cannot start. A feasibility check on 2026-10-08 showed the
interpreter can be compiled from a fixed file list with a checked-in `config.h`, without its
`configure` and `make`, and still passes its own core test suite.

## What Changes

- Vendor the needed part of the Ruby 1.8.7-p374 release under `third_party/ruby-1.8.7/`, with
  its licence files, a manifest of file hashes and a log of local patches.
- `vkxp-ruby-sys` builds the interpreter and its `Zlib` extension as static libraries from the
  build script, with pinned compiler flags and hidden symbols, and links a statically built
  zlib. No system Ruby, `configure` or `make` is involved.
- One local patch: `Zlib.crc_table` returns correct values on 64-bit.
- Hand-written FFI declarations for the interpreter's C API, and helpers that start the
  interpreter on its own thread with a known stack, make `Zlib` available without `require`,
  restore the process's signal handling and run closures on that thread.
- Ruby's own core tests (625 tests), its `Zlib` tests and its sample test run under
  `cargo test`, because a wrong compiler flag does not fail the build, only these tests.

Out of scope: the RGSS classes, loading game scripts, the policy on dangerous functions,
Oniguruma (the stock regular-expression engine is used for now), targets other than x86_64
Linux with glibc.

## Capabilities

### New Capabilities

- `embedded-ruby`: the embedded Ruby 1.8.7 interpreter: how it is built, what must be true of
  it at start-up, and how host code enters and leaves it safely.

### Modified Capabilities

None.

## Impact

- New top-level directory `third_party/ruby-1.8.7/` (about 160 files, 3.4 MB) under Ruby's own
  licence; the repository's MIT licence does not cover it, and `README.md` says so.
- Crate `vkxp-ruby-sys`: build script, FFI, thread helper, tests. New build dependencies `cc`
  and `libz-sys` (static). A C compiler (gcc or clang) and the development files of libcrypt
  become build prerequisites.
- `cargo test --workspace` grows by Ruby's test suite, a few seconds when run in parallel.
- `Cargo.toml`, `Cargo.lock` and `README.md` (prerequisites, third-party notice) change.
