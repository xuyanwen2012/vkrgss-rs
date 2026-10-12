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

None yet.

## Checked

To be filled by the run that implements this: the values of `n`, the time of each step, the
findings and their outcome, and that the aarch64 job was not run.
