# Design

## Context

- `crates/vkxp-ruby-sys/tests/sanitized.rs` builds the interpreter and Oniguruma with clang,
  AddressSanitizer and UndefinedBehaviorSanitizer from the file lists the build script
  exports; `tests/common/mod.rs` has the shared parts; `stress-the-collector-and-try-aarch64`
  added `VKXP_GC_STRESS` and `cargo xtask stress`.
- The collector scans the stack conservatively; the exclusions that make it run under
  AddressSanitizer are patches 18 and 19.
- Deep nesting recurses in the parser and in `Marshal.load`; the executable runs the
  interpreter on a thread with a large stack and a bounded depth (requirement "Dedicated
  thread and bounded recursion").
- The regular expression probe (`tests/regex.rs`) and the other measured rules pin
  behaviour of the original runtime that differs from Ruby 1.8.7 in places.

## Decisions

### Targets

One C file per target under `crates/vkxp-ruby-sys/fuzz/`, each with
`LLVMFuzzerTestOneInput`, linked with the interpreter built as for the sanitized run plus
libFuzzer's coverage instrumentation. The interpreter is initialised once; each input runs
under a protected call, and a Ruby exception is not a finding.

- `marshal`: `Marshal.load` of the bytes.
- `parse`: the bytes compiled as a script and not run.
- `regex`: the bytes up to the first line break compiled as a pattern, the rest matched
  against it.

Seeds are small files under `crates/vkxp-ruby-sys/fuzz/seeds/<target>/`, written for this
change (dumps of plain values, short scripts, patterns from the regular expression probe).
No file of a game or of Enterbrain is used.

### The command

`cargo xtask fuzz <target> [--seconds N]` builds the target under the target directory and
runs it there with the seeds, a default of 60 seconds, an input size limit, and a per-input
time limit. It needs clang with its sanitizer runtimes and says so when they are missing.
The corpus a run grows stays under the target directory. It is not a gate step and not a
CI job. `cargo test -p xtask` covers how the command is built.

### Findings of the first run

Each target runs for 20 minutes on the development machine. What counts:

- A sanitizer report or a crash on an input: fixed in the vendored tree as a recorded patch
  when the fix is a few lines, with the input (reduced) as a test that runs in the normal
  test step; otherwise listed under "Follow-ups" with the report and the input kept beside
  the seeds.
- Stack exhaustion by nesting: not a finding of this change; the target gives such inputs a
  size limit that avoids it, and "Checked" says at which depth it shows.
- A per-input time-out in `regex`: not a finding; backtracking is the engine's nature.
- Memory that grows without bound across inputs because the harness keeps something alive:
  a defect of the harness, fixed there.

A fix must not change what `tests/regex.rs` and the other measured-rule tests pin. Where a
defect is behaviour the original runtime has too, it stays and is listed.

### Review of later fixes

Sources: the security advisories of ruby-lang.org since June 2013, the patches Debian and
Red Hat carried for their `ruby1.8` packages after p374, and the CVE entries and change log
of Oniguruma after 5.9.6 (the series of 2017 and of 2019 to 2020 in particular). For each
entry `docs/vendored-security.md` has a row: identifier, the file and function concerned,
whether that code is in the vendored tree and built, whether a script or a data file of a
game reaches it, and the outcome: backported (patch number), does not apply (why), or left
(why, for example because the fix changes behaviour the original runtime has).

A backport is a recorded patch, with a test where a few lines can show the defect. Code
that vkxp does not build (network, `ext/` other than zlib, Windows) is "does not apply"
without further study. If the network is not available to the run, the table is made from
what the implementer can establish from the trees alone, and "Checked" says that the
sources were not consulted.

## Risks

- [A backport changes behaviour a game relies on] → the measured-rule tests and the
  upstream suites run in the gate; the owner runs the seven-game regression at the merge.
- [Twenty minutes find little] → the command stays, and longer runs are a matter of time.

## Follow-ups

None yet.

## Checked

To be filled by the run that implements this: the time and the number of executions of
each target, the findings and their outcome, and how the review's sources were consulted.
