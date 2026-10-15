# Design

## Context

About 78 files use `testing::`: `eval` 46 call sites, `start_as` 43, `host` 38, `game_dir`
26, `run` 12, a few others. Tests of RGSS1 mostly never call `start_as` and rely on the
fallback. `vkrgss-binding-rgss3` builds the test files of `vkrgss-binding` a second time over
Ruby 1.9.2; the lists of tests of both packages must stay as they are (the requirement "The
tests of the binding run over both interpreters" and the allow-list over Ruby 1.9.2 depend on
the names).

## Decisions

- **Handle.** `pub fn start_as(runtime: Runtime) -> &'static Harness` (or a `Copy` handle
  type that wraps it). `Harness` gets the methods. The free functions that reach the
  interpreter are removed, not deprecated, so a missed call site is a compile error. Helpers
  that need no interpreter (`pattern`, `color`, `pending`, `CellBackend` and the like) stay
  free.
- **Per-file helper.** A test file may define one small function such as
  `fn harness() -> &'static Harness { testing::start_as(Runtime::Rgss1) }` and call it in
  each test, to keep the diff mechanical. The runtime stays visible in the file.
- **Test names unchanged.** No test is renamed, added or removed by the move, so the lists of
  both packages and the allow-lists stay valid.
- **Directory per process.** The name of the generated directory gains the process id. The
  directory is still removed and created at start; old directories of dead processes are not
  cleaned up by this change beyond what `cargo clean` does (a follow-up if they pile up).
- **Error message.** Asking for a second runtime in one process keeps panicking, and the
  message names both runtimes.

## Out of scope

Introducing cargo-nextest into the gate; changes to what any test checks.

## Follow-ups

None yet.
