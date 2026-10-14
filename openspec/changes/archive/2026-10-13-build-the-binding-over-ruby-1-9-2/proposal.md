# Build the binding over Ruby 1.9.2

## Why

A game of RPG Maker VX Ace needs the RGSS classes over Ruby 1.9.2. The interpreter is built
(`vkxp-ruby192-sys`, change `vendor-ruby-1-9-2`), and decision 5 of `plan-vx-ace` says how
the classes get there: one source, compiled a second time as a package of its own over the
second interpreter crate, with what differs behind the API of the interpreter crates or a
check at run time. Today that source names seven things only Ruby 1.8.7 has, makes every
string without saying whether it is text or bytes, and its tests (22 files, about 260
tests) run over one
interpreter. Nothing of RGSS3 can be built, measured against the original or stressed
before the same classes compile and pass their tests over both.

## What Changes

- A second package, `vkxp-binding-rgss3`, compiles `crates/vkxp-binding/src` over
  `vkxp-ruby192-sys`. It has no source of the classes. On a target for which the second
  interpreter is not built it is empty.
- The two interpreter crates get one common surface for what the binding uses: a constant
  that says which interpreter it is, the options of an interpreter for a runtime, the main
  object, the accessors of strings and arrays, a constructor for text beside the one for
  bytes, and the mark of the game's scripts. The additions to `vkxp-ruby-sys` are small
  functions that change nothing of Ruby 1.8.7.
- Every string the binding creates is classified as text or bytes and made through one of
  two functions. Over Ruby 1.8.7 both give the same string.
- The preparation of the interpreter sets `$KCODE` and empties the load path over Ruby
  1.8.7 only.
- The unit tests and the integration tests of the binding run once per interpreter from
  the same files. What cannot pass over Ruby 1.9.2 yet, because it expects behaviour of
  RGSS1 or RGSS2 or wording of Ruby 1.8, is named in an allow-list with its reason; a
  listed test is still run, alone, and the run fails when it starts to pass.
- `cargo xtask stress` gets a step that runs the tests of the second package with the
  collector at every allocation.
- A test of the second package runs a script of RGSS3 flavour through the classes: a
  bitmap, a sprite, a Fiber, one extracted frame.
- Not in this change: the guest executable of RGSS3 and its start by the program, any
  behaviour of the classes that RGSS3 has and RGSS2 has not, the script loader and
  `load_data` of RGSS3, `Win32API` and `DL` over Ruby 1.9. XP and VX do not change.

## Capabilities

### New Capabilities

- `binding-over-two-interpreters`: the one source and its two packages, the common surface
  of the interpreter crates, text and bytes, the preparation by interpreter, the tests over
  both interpreters with their allow-list, the stressed run and the script of RGSS3
  flavour.

### Modified Capabilities

- `workspace-structure`: the crates `vkxp-ruby192-sys` and `vkxp-binding-rgss3` in the
  lists of crates and in the dependency rules.
- `embedded-ruby-1-9-2`: the Rust API of the crate, which gains the items of the common
  surface.

## Impact

- New: `crates/vkxp-binding-rgss3` (`Cargo.toml`, `build.rs`, `tests/`).
- `crates/vkxp-binding`: `src/lib.rs` (one attribute), `src/rb.rs`, `src/runtime.rs`,
  `src/testing.rs`, the call sites that make strings, the prelude files; in `tests/` the
  edits that the specification names and no expected value.
- `crates/vkxp-ruby-sys` and `crates/vkxp-ruby192-sys`: the additions of the common
  surface, their tests, the crate documentation.
- `crates/vkxp-guest-main`: the options of the interpreter through the common constructor.
- `xtask/src/deps.rs`, `xtask/src/stress.rs`, `xtask/src/stress.skip`, tests of `xtask`.
- `Cargo.toml` (members, workspace dependencies, the name of one `cfg`), `Cargo.lock`,
  `AGENTS.md`, `README.md`, `docs/architecture.md`, `docs/decisions.md`,
  `docs/development.md`.
- The gate: the lint step and the test step also cover the second package, so the tests of
  the binding are built and run twice; the documentation step does not render it.
- Not touched: `crates/vkxp`, `vkxp-guest`, `vkxp-core`, `vkxp-proto`, the host crates,
  `third_party/`, the release package.
