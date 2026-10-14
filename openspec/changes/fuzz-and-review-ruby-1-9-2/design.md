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

## Follow-ups

Filled by the implementation.
