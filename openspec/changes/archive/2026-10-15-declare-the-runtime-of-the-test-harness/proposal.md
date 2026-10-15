# Declare the runtime of the test harness

## Why

The test harness of the binding (`crates/vkrgss-binding/src/testing.rs`) starts one
interpreter per test process. Its runtime comes from `start_as`, and when no test has called
`start_as` yet, `harness()` falls back to `Runtime::Rgss1` (`unwrap_or(Runtime::Rgss1)`). The
first test that touches any function of the harness therefore decides the runtime of the
whole binary, and which test runs first depends on the order and the threads of the test
runner. On 2026-10-14 this made a test of Win32API over RGSS3 fail: a test asked for
`game_dir()` before another called `start_as(Runtime::Rgss3)`. The fix
(`42c05948`) added one `start_as` call to that file; every other file still depends on order.

The generated game directory is named after the test executable, so two processes of the
same executable (as cargo-nextest runs them, one process per test) would remove each other's
files.

## What Changes

- `start_as(runtime)` returns a handle of the started harness, and the functions that need
  a started interpreter (`eval`, `run`, `game_dir`, `host`, `bitmap_count`, `with_scene` and
  the like) become methods of that handle or take it. Without a handle, no test can reach the
  interpreter, so the compiler guarantees that the runtime was declared first.
- The fallback to RGSS1 is removed. Asking for a second runtime in the same process still
  panics, as now.
- The directory of generated files is named after the process as well as the executable.
- Every test in `vkrgss-binding`, `vkrgss-binding-rgss3` and elsewhere that uses the harness
  is moved to the handle; tests that relied on the fallback declare RGSS1.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `binding-over-two-interpreters`: the harness is reached only after its runtime is
  declared.

## Impact

`crates/vkrgss-binding/src/testing.rs` and `testing/`, the unit tests of `vkrgss-binding`
that use the harness, the integration tests of `vkrgss-binding` and
`vkrgss-binding-rgss3`, and any other crate whose tests use `vkrgss_binding::testing`.
No change to the binding's behaviour in a game; tests only. `docs/development.md` if it
describes the harness.
