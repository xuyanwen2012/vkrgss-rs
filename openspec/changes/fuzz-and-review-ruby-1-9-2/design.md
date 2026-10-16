# Design

## Context

- `crates/vkrgss-ruby-sys` has what this change repeats for the other tree:
  `tests/sanitized.rs` with `tests/sanitized.allow`, the targets `fuzz/marshal.c`,
  `fuzz/parse.c`, `fuzz/regex.c` with `fuzz/harness.h`, seeds under `fuzz/seeds/`, fixed
  inputs under `fuzz/fixed/`, and `tests/fuzz.rs`, which replays the fixed inputs.
- `xtask/src/fuzz.rs` builds and runs a target by name; it knows only the three targets of
  Ruby 1.8.7.
- `crates/vkrgss-ruby192-sys` builds Ruby 1.9.2 with its own in-tree regular expression
  engine (a fork of Oniguruma 5.9.x) and runs the upstream suite, also under `GC.stress`
  (`tests/upstream.rs`, `tests/upstream_stress.rs`, `tests/upstream.allow`). It is built
  only for x86_64 Linux with glibc.
- `docs/vendored-security.md` reviews Ruby 1.8.7-p374 and Oniguruma 5.9.6, and since 0.3.1
  has a short section on Ruby 1.9.2 that says the review is open.
- The measured rules of RGSS3 (Marshal as the original writes it, random numbers, Float
  text, encodings) are pinned by tests of `vkrgss-ruby192-sys` and of
  `vkrgss-binding-rgss3`.

## Decisions

### Sanitized run

`crates/vkrgss-ruby192-sys/tests/sanitized.rs` builds the interpreter from the file list
the build script exports, with clang, `-fsanitize=address,undefined` and the exclusions the
conservative collector needs (as patches of the tree, if 1.9.2 needs its own), and runs the
upstream suite with it. It is `#[ignore]`d, as 1.8.7's is, and runs with `--ignored`. An
allow-list `tests/sanitized.allow` names the reports accepted and why; every report not on
it fails the run. A report is fixed as a recorded patch where the fix is a few lines, and
listed otherwise.

### Targets

`crates/vkrgss-ruby192-sys/fuzz/` with the same three targets, reusing the shape of
`harness.h` (interpreter initialised once, each input under a protected call, a Ruby
exception not a finding):

- `marshal`: `Marshal.load` of the bytes.
- `parse`: the bytes compiled as a script and not run, with the source encoding UTF-8 as
  vkrgss compiles scripts of RGSS3.
- `regex`: the bytes up to the first line break compiled as a pattern in UTF-8, the rest
  matched against it.

`cargo xtask fuzz` names them `marshal192`, `parse192` and `regex192`; the names of the
1.8.7 targets stay. Seeds are written for this change; no file of a game or of Enterbrain
is used.

### First run and findings

Each target runs for 20 minutes. The rules of the 1.8.7 change apply unchanged: a
sanitizer report or crash is fixed as a recorded patch with the reduced input as a test
under `fuzz/fixed/` (replayed by `tests/fuzz.rs` of this crate in the normal test step), or
listed under "Follow-ups"; stack exhaustion by nesting and regex time-outs are not
findings; a leak of the harness is fixed in the harness.

### Review of later fixes

Sources: the security index of ruby-lang.org from August 2010 to the date of the review,
the 1.9.2 patch releases (p136 to p330) and their change logs, the patches Debian carried
for `ruby1.9.1` (which shipped 1.9.2 and 1.9.3), and for the in-tree engine the Oniguruma
and Onigmo CVEs already listed for 5.9.6. Each entry gets a row in a new section of
`docs/vendored-security.md` in the form of the existing tables: identifier, code, built,
reached by (data, script, not at all), outcome. Candidates to check first, from the audit:
CVE-2011-2686 (random seed after `fork`, which save states use), CVE-2011-4815 (hash
flooding; the seed is fixed on purpose, so the row says why), CVE-2014-4975 and
CVE-2018-8778 (`pack`/`unpack`), CVE-2017-0898 (`sprintf`), CVE-2014-3916 (`str_buf_cat`),
the 2017 and 2019 Oniguruma series against the in-tree engine, and the `Marshal` and
`String#to_f` fixes of 1.9.3.

A fix that would change behaviour measured on the original runtime of RGSS3 is "left",
with the reason, as for 1.8.7. Code vkrgss does not build (network, `ext/` except what the
crate builds, Windows) is "does not apply".

## Risks

- [A backport changes behaviour a game relies on] → the measured-rule tests, the upstream
  suites (also under `GC.stress`) and the saves of the A3 record run in the gate; the owner
  reruns the two A3 games at the merge.
- [The sanitizer and the copying fibers of 1.9.2 disagree] → fibers copy the machine stack;
  AddressSanitizer may report those copies. Such reports go on the allow-list with the
  reason, or the copy routine is excluded from instrumentation as the collector's are.

## Checked

By the run that implemented this, on 2026-10-15, on the development machine (12 hardware
threads, clang 22.1.8 with `compiler-rt`).

### The sanitized run

`cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored` builds the interpreter, its
encodings, `Zlib` and the test executable again with clang and both sanitizers (the build
script now exports its file lists) and runs `bootstraptest` and `test/ruby` with the list of
`tests/upstream.allow`; it takes about two minutes. What the run needed, beside the two
exclusions the collector and the copying Fibers need (patches 19 and 20):

- `use_sigaltstack=0`: the interpreter gives every thread an alternate signal stack of its
  own, and at the end of a thread AddressSanitizer unmaps whichever is set, which failed for
  that buffer of `malloc` and ended the process after every `Thread.new`.
- `handle_segv=0` and `disable_coredump=0`: `test_segv_test` and `test_execopts_rlimit` test
  the interpreter's own handler and the limit of core files.
- The options are also compiled into the executable (`__asan_default_options`): the suites
  start interpreters with an emptied environment, in which leak detection ran at exit, and
  its helper process was a second child that `test_wait_and_sigchild` saw.
- No stack trace from UndefinedBehaviorSanitizer: printing one starts `llvm-symbolizer` as
  a child, and `Process.wait` in `test_process.rb` waited for it forever.

It found one defect, a heap overflow of one byte in the conversion `%a` (patch 21). The
allow-list accepts the misaligned loads of the regular expression engine, two calls of
`memcpy` with a null pointer and a length of zero, and one pointer that is computed one
slot before an array and never used.

### First run

As for Ruby 1.8.7, the first run was a series: a run until the first report, the fix, the
next run, about a dozen runs of 30 seconds to 10 minutes, before the run of 20 minutes that
each target then completed without a finding:

| Target | Time | Inputs run | Put aside, not findings |
|---|---|---|---|
| `marshal192` | 1201 s | 2,109,675 | none |
| `parse192` | 1201 s | 5,126,026 | 1 process at the memory limit |
| `regex192` | 1205 s | 2,647,209 | 61 inputs at the time limit of 3 s |

The run of `regex192` was made before patch 28, which changes only the marking of a node of
the parser. The second run, 5 minutes per target with `cargo xtask fuzz <target> --seconds
300` on the trees with every patch of this change: `marshal192` 442,706 inputs, `parse192`
736,312, `regex192` 372,035 (12 inputs at the time limit); each ended with status 0. `cargo
xtask fuzz <target> --seconds 30` ended with status 0 after 38 to 39 seconds, build
included, and `cargo xtask fuzz nothing` with status 2 and the names of the six targets.
These short runs were made after the fixes of the first run, because the first short run
of `marshal192` found the defect of patch 22 within seconds.

Findings, each fixed by a recorded patch of `third_party/ruby-1.9.2/PATCHES.md` with its
reduced input under `crates/vkrgss-ruby192-sys/fuzz/fixed/`, which
`the_inputs_of_fixed_findings_no_longer_fail` of `tests/fuzz.rs` replays in the normal test
step:

| Found by | Defect | Patch |
|---|---|---|
| `marshal192` | a dumped Time with a month above 12 indexes the tables of the months past their end | 22 |
| `parse192` | the rule of a regexp literal takes an element of the list of pieces for a string (`/#$y #{"s"}#{"t"}/`, a valid script) | 23, 24 |
| `parse192` | `assign_in_cond` reads a value that is null after a syntax error | 23, 24 |
| `regex192` | a backslash before a character cut off by the end passes the check of the pattern, and the engine reads past it | 25 |
| `regex192` | a wide character after a range and a hyphen in a class sets a bit past the bit set (upstream #21 and CVE-2017-9226) | 26 |
| `regex192` | groups that refer to each other make the measuring of lengths recurse without end (upstream 7fe8c3f, which the tree of 1.8.7 left) | 27 |
| `parse192` | the collector follows the name in a node of `::Name` as a pointer, and `class ::c` leaves a name there that is not skipped | 28 |

The input of patch 28 does not fail on its own, only after a collection at the right
moment; it was reduced with the collector stressed, and the test compiles it under
`GC.stress`. Every input gave its report in a build from before its fix: the build of the
target that found it, in which it was reduced, or the sanitized interpreter.

## Follow-ups

Findings and open points that this change lists and does not fix:

- **Stack exhaustion by nesting** is not a finding, as for Ruby 1.8.7: deeply nested groups
  or classes in a pattern (upstream #40, CVE-2019-16163, CVE-2017-6181) and deeply nested
  values in `Marshal.load` exhaust the stack and end the process; the input limits of the
  targets keep inputs far from those depths.
- **Two new errors are not measured on the original runtime.** A dumped Time with a month
  above 12 is now a `TypeError`, and a backslash before a character of several bytes cut
  off by the end of a pattern a `RegexpError`; the release, and so the original, read past
  a table and past the pattern for them. Neither can be written by the original runtime
  itself.
- **The parser does not free the tables of the scopes a syntax error leaves open**, and a
  symbol is never freed: a process of `parse192` reaches libFuzzer's memory limit after a
  few million inputs and is replaced, as for 1.8.7.
- **The targets do not select another encoding than UTF-8**: patterns in UTF-16LE or
  Shift_JIS, and the recompilation of a pattern's source in `Regexp#to_s`, are not fuzzed.
- **libFuzzer's fork mode is not used**, as for 1.8.7; the test starts a new process when
  one ends on the time or memory limit.
- **The sanitized run has no stressed variant** for Ruby 1.9.2: the stressed suites of that
  tree (`tests/upstream_stress.rs`) run in the ordinary build. A stressed sanitized run, as
  `cargo xtask stress` has for Ruby 1.8.7, would be new machinery.
- **Patch 20 takes the red zones off the frames that are live at a Fiber switch**, so the
  sanitized run can miss a stack overflow in them; it cannot report a false one.
