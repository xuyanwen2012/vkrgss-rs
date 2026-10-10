# Proposal

## Why

- The interpreter is C from 2013, built for 64-bit with patches, and it has had a real memory
  error on 64-bit already (the first patch of the vendored tree). Its 625 upstream tests run
  in the gate, but only for their results: nothing looks for invalid memory accesses or
  undefined behaviour that happen not to change a result.
- The parser of the encrypted archive reads lengths and offsets from a file a game ships. It
  has hand-written tests for a wrong header and a truncated file, and none with arbitrary
  bytes.
- The dependencies are not checked against security advisories or for their licences.

## What Changes

- An opt-in run builds the interpreter and Oniguruma with AddressSanitizer and
  UndefinedBehaviorSanitizer and runs the upstream suites under them. What it finds is fixed
  when the fix is small and clear, and recorded otherwise.
- Property tests feed the archive parser arbitrary and mutated bytes: it returns an error or a
  directory, and does not panic, hang or allocate out of proportion.
- `cargo xtask audit` runs `cargo deny` with a committed configuration (advisories, licences,
  sources).

Out of scope: adding either to the merge gate (the sanitizer run is slow, the audit needs the
network); fuzzing with a fuzzer; sanitizing Rust code; fixing every finding regardless of
size.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: the sanitizer run.
- `game-files`: arbitrary archive bytes.
- `workspace-structure`: the audit command.

## Impact

- `crates/vkxp-ruby-sys` (build script and tests; the vendored tree only for a recorded
  patch), `crates/vkxp-vfs` (tests, and the parser if a test finds something), `xtask`,
  `deny.toml`, `README.md`.
