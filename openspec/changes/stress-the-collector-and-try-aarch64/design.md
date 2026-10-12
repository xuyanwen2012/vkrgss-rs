# Design

## Context

- `gc.c` has `ruby_gc_stress`; with it set, `ruby_xmalloc` collects on every call. Object
  allocation (`rb_newobj`) collects only when the free list is empty, so `GC.stress` alone
  does not collect at every object.
- `crates/vkxp-ruby-sys/tests/sanitized.rs` builds the interpreter with AddressSanitizer and
  UndefinedBehaviorSanitizer and runs the upstream suites; `tests/common/mod.rs` holds the
  runner; `tests/sanitized.allow` the accepted reports. The collector's stack scan and the
  thread switch are excluded from instrumentation (patches 18 and 19).
- A string's buffer is freed with `free` when the string is collected, so under
  AddressSanitizer a use of the buffer of a collected string is a reported
  use-after-free. The slot of a collected object is not poisoned.
- The CI workflow has one job, `gate`, on `ubuntu-24.04`.

## Decisions

### The switch

`VKXP_GC_STRESS=<n>`, read once when `vkxp-ruby-sys` initialises an interpreter: with `n`
of 1 or more the collector runs at every n-th allocation, counting object allocations and
`ruby_xmalloc` calls together; unset, empty or 0 changes nothing. The counting in
`rb_newobj` is a recorded patch to `gc.c`, one predictable branch when the switch is off.
The variable is read in every build, also in the release executable, so that a game can be
run under stress; it is named in `docs/development.md` only.

A collection that the stress asks for while collections are disabled (`dont_gc`,
`during_gc`) does not happen, as for any other request.

### The runs

`cargo xtask stress` runs, in this order, and reports all that fail:

1. the upstream suites in the sanitized build with the switch on (a second ignored test
   beside `sanitized`, sharing its build and its allow-list);
2. `cargo test -p vkxp-binding --locked` with the switch on;
3. the tests of `vkxp` that start a guest (`end_to_end`), with the switch on and
   `VK_LOADER_DRIVERS_SELECT` passed through.

Step 1 uses the smallest `n` with which it ends within about half an hour on the
development machine; steps 2 and 3 use 1 unless that takes more than about half an hour,
then the smallest `n` that does not. The values are constants beside the command and stated
under "Checked". The command is not a gate step and not a CI job.

Poisoning the slots of collected objects for AddressSanitizer is not part of this change.

### Findings

A value lost under stress is a defect also without it. It is fixed where it is found, with
a guard of the kind the tree uses (`RB_GC_GUARD`-style volatile reference, or the binding's
rooting helpers), as a recorded patch when it is in the vendored tree, when the fix is a few
lines; each fix gets a line under "Checked" with the test that showed it. A finding that
needs more is listed under "Follow-ups" with its report and the test is named in a skip
list beside the command, with the reason. A test that fails under stress for a reason that
is not a lost value (it measures allocations or time) goes on the skip list with that
reason.

### aarch64

A job `gate-aarch64` in `ci.yml`: the same steps as `gate` on `ubuntu-24.04-arm`, with
`continue-on-error: true`. Nothing in the build is changed for aarch64 by this change,
except that the build script must not refuse the target if it does today. The run that
implements this cannot start the job; "Checked" says so. `docs/compatibility.md` says that
x86_64 is the supported architecture and that the aarch64 job shows what works there;
`docs/decisions.md` keeps its risk line.

## Risks

- [The stressed run is slow] → every n-th allocation, outside the gate.
- [The aarch64 job is red for a long time] → it does not fail the workflow, and the
  documents do not claim support.

## Follow-ups

- The runs find a lost value only where a test executes the code and only where the loss
  is visible: a freed string buffer under AddressSanitizer in the first step, a wrong
  result or a crash in the other two, which are not sanitized. The slot of a collected
  object is not poisoned ("The runs"), and the binding's tests were not run in a sanitized
  build. That no finding came out of the first run says this much and no more about the 42
  `unsafe` places of `vkxp-binding`.
- The stressed runs used one compiler each (gcc for the binding and the executable, clang
  for the sanitized build). What a collector finds on the stack depends on the compiler;
  `docs/development.md`, "Testing with another compiler", has the commands to combine with
  `VKXP_GC_STRESS`.

## Checked

By the run that implemented this, on 2026-10-12, on the development machine (12 hardware
threads, gcc for the workspace, clang for the sanitized build).

The switch:

- `rb_newobj` of this patch level already collects under `GC.stress`, other than "Context"
  says; the patch (27, `gc-stress-every-nth`) therefore adds only the counting, in
  `rb_newobj`, `ruby_xmalloc` and `ruby_xrealloc`. `ruby_xrealloc` is counted as well
  because it is an allocation of memory in the words of the requirement and collects under
  `GC.stress`: growing a string is where patch 24 found its fault.
- This Ruby has no `GC.count`; the patch adds the counter `ruby_vkxp_gc_count`, which the
  tests `gc_stress_on` and `gc_stress_off` of `vkxp-ruby-sys` read. With the switch on, 100
  objects give at least 100 collections; without it, none.
- A value of the variable that is not a number is refused when the interpreter starts
  (`StartError::InvalidGcStress`), so that a mistyped value is not a run without stress.
- The sanitized executable is Ruby's own command line, which `vkxp-ruby-sys` does not
  start; the `Init_ext` that the test writes for it reads the variable and calls the same
  function.

The runs, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress`, all steps passed:

| Step | `n` | Time |
|---|---|---|
| `suites` | 32 | 1603.6 s |
| `binding` | 1 | 30.7 s |
| `guest` | 1 | 5.6 s |

- `n` of the first step: one file decides its time, `test/ruby/test_integer.rb`, a single
  process with 1.6 million assertions on big numbers. Alone it took 417 s with `n` of 512,
  483 s with 256, 655 s with 128 and 991 s with 64; in the step, beside the other files,
  1518 s with 32. With 16 it would take about 50 minutes. Every other file ends within ten
  minutes with 64. The time limit of one test process is 45 minutes in the stressed test.
- "A lost value shows": `a_lost_value_is_reported_under_collector_stress` in
  `tests/sanitized.rs` calls a C function of the test executable that keeps only the buffer
  pointer of a fresh string across an evaluation. With `VKXP_GC_STRESS=1` the process ends
  with `AddressSanitizer: heap-use-after-free`; without the switch it prints the string.
  The first step of the command runs this test too.
- Whether the switch reaches a guest process was checked with a value that is not a number:
  51 of the 59 tests of `end_to_end` then fail at the start of the interpreter.

Findings: none. No test failed under stress and the sanitizers reported nothing outside the
allow-list, so no guard was added, `vkxp-binding` is unchanged and the skip list
(`xtask/src/stress.skip`) is empty. What the runs do not show is under "Follow-ups".

Without the switch, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-ruby-sys
-p vkxp-binding -p vkxp --locked` passes, and so does the sanitized run without stress.
