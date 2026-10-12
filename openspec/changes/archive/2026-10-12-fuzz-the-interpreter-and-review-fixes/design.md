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

Findings and open points that this change lists and does not fix:

- **Text that is not valid in the encoding, in Oniguruma.** The engine assumes that a
  pattern and a subject are valid in their encoding, and upstream never changed that. The
  patches of this change bound the reads that the targets reached, for UTF-8, and refuse a
  pattern that ends inside a multibyte character (upstream's own fix). Reads a few bytes
  past a string remain possible elsewhere for bytes that are no character, in Shift_JIS and
  EUC-JP in particular: the targets set `$KCODE` to UTF-8 as vkxp does and never select
  another encoding. A target per encoding, or a check of validity in front of the engine,
  would be new machinery; the second would also refuse patterns the original accepts.
- **Two new errors are not measured on the original runtime.** A pattern that ends inside
  a multibyte character is now a `RegexpError` ("too short multibyte code string"), and an
  octal escape above `\377` is one too ("too big number"); the vendored engine read out of
  bounds for both. What the older engine of the original does with them was not measured.
  `tests/regex.rs` and the other measured-rule tests pass unchanged.
- **Stack exhaustion in the engine.** `(\2)(\1)` recurses without end when it is compiled
  (upstream 7fe8c3f, a rewrite of about 540 lines), and nothing limits the nesting of
  groups (upstream 6b68ebe and CVE-2019-16163, a limit with a new error code over four
  files). Both exhaust the stack and end the process. Stack exhaustion is not a finding of
  this change; the rows of `docs/vendored-security.md` say "left".
- **The backward search reads one byte after the subject** (`String#rindex` with a
  pattern, found by the review, not by a target). For a Ruby string that byte is the
  terminator. Upstream's fix is part of the rewritten search of 6.x, and a smaller one
  moves where the backward search starts, which was never measured on the original.
- **`code_to_mbc` of EUC-JP** has the shape upstream fixed in 86f9fc7; no pattern was found
  that reaches it with a code point outside the encoding, and nothing here tests EUC-JP.
- **CVE-2018-8780 and CVE-2019-15845 are left**: a path, and a pattern of `File.fnmatch`,
  are used up to their first NUL byte, as in all of Ruby 1.8. The reason given in
  `docs/vendored-security.md` (scripts pass buffers filled by `Win32API`) is an argument,
  not a measurement on the original.
- **Two defects without a published fix for 1.8**, seen while reading for the review and
  reached only by a script: the width arithmetic of `sprintf` (`"%2147483647f"` skips the
  growth of the buffer; upstream r59410 is related), and the `alloca` of `encodes` in
  `pack.c`, whose size comes from the count of `m` and `u` (fixed for 1.9 in r19139).
- **One report was seen once and not explained.** Early in the first run, before the
  parser fixes of patches 30 and 31, the `parse` target ended on a corrupted free list of
  the collector (`gc.c`, `rb_newobj`: a misaligned next pointer, equal to the name of a
  source file). The input alone did not reproduce it, nor did the corpus of that time in
  one process. After the fixes the target ran 20 minutes with `rb_gc_force_recycle` made
  to abort on an object that is already free, which is how a double recycle would corrupt
  the list, and did not abort; the out-of-bounds write of `yyerror` that patch 30 removes
  is the likely cause, but that was not shown.
- **The parser does not free the tables of the scopes a syntax error leaves open**, about
  260 bytes for each failed compilation, and a symbol is never freed. A process of the
  `parse` target therefore reaches libFuzzer's memory limit after a few hundred thousand
  inputs and is replaced; that is the interpreter's own behaviour and stays.
- **libFuzzer's fork mode is not used.** With `-fork=1` the parent process of clang 22.1.8
  ended with a segmentation fault in its own code after a few jobs, with and without an
  interpreter started in it. The test that runs a target starts a new process itself when
  one ends on the time or memory limit.
- **`cargo test -p vkxp-ruby-sys --test sanitized -- --ignored`**, the verification of task
  3.1, includes a test that fails unless `VKXP_GC_STRESS` is set, as it did before this
  change; it was run with `VKXP_GC_STRESS=32`, the value `cargo xtask stress` sets.

## Checked

By the run that implemented this, on 2026-10-12, on the development machine (12 hardware
threads, clang 22.1.8 with `compiler-rt`), in the sanitized build.

### How the targets run

- A target is one C file in `crates/vkxp-ruby-sys/fuzz/`; the start of the interpreter and
  the options of the sanitizers are in `harness.h`, which the three include. The ignored
  test of `tests/fuzz.rs` builds and runs a target, and `cargo xtask fuzz` runs that test,
  as `cargo xtask stress` runs the stressed tests.
- Limits: inputs of 4096 bytes for `marshal` and `parse` and 512 bytes for `regex`; 10
  seconds for one input, 3 for `regex`; libFuzzer's memory limit of 2 GiB; one allocation
  of at most 256 MiB.
- A process that ends on the time limit or on the memory limit is not a finding; the test
  starts the next one (see "Follow-ups" for fork mode).
- Undefined behaviour ends a run. Two checks are off: a null pointer given to `memcpy`
  with a length of zero, and, in the files of the engine, misaligned loads; the allow-list
  of the sanitized run accepts both with its reasons.
- Stack exhaustion by nesting, measured with the built targets and a stack of 8 MiB:
  `Marshal.load` at 21,815 nested arrays (an input of 43,633 bytes) and at 21,665 nested
  hashes; the engine at 3,190 nested groups (6,383 bytes) and 3,490 nested classes; the
  parser not at all up to 262,144 nested parentheses, brackets or unary operators, because
  its own stack limit makes a syntax error first. The input limits keep every target below
  2,048 levels.

### First run

Each target found defects within seconds of its first start, so the run was a series: a
run until the first report, the fix, the next run. About twelve such runs of 20 seconds to
4 minutes per target came before the run of 20 minutes that the design asks for, which
each target then completed without a finding:

| Target | Time | Inputs run | Put aside, not findings |
|---|---|---|---|
| `marshal` | 1201 s | 1,765,738 | none |
| `parse` | 1201 s | 4,187,341 | 16 processes at the memory limit |
| `regex` | 1215 s | 1,164,877 | 61 inputs at the time limit of 3 s |

The second run, 5 minutes per target with `cargo xtask fuzz <target> --seconds 300` on the
final trees, while the sanitized suites ran under collector stress on the same machine:
`marshal` 178,374 inputs, `parse` 560,308 (2 processes at the memory limit), `regex`
135,901 (8 inputs at the time limit); each ended with status 0. `cargo xtask fuzz <target>
--seconds 30` ended with status 0 after 35, 36 and 48 seconds, and `cargo xtask fuzz
nothing` with status 2 and the names of the three targets.

### Findings and their outcome

All are fixed by recorded patches; the numbers are those of `PATCHES.md` of each tree.

| Found by | Defect | Patch |
|---|---|---|
| `marshal`, `parse` | `rb_intern` reads past a name that ends inside a multibyte character | Ruby 28, 29 |
| `marshal` | `is_special_global_name` (and `rb_symname_p`, by reading) the same | Ruby 30, 31 |
| `parse` | `yyerror` copies and marks past the line of the error | Ruby 30, 31 |
| `parse` | the rules of `..` and `...` and `assign_in_cond` read a null operand | Ruby 30, 31 |
| `regex`, `marshal` | `rb_reg_expr_str` copies past the source of a pattern | Ruby 32 |
| all three | `mbc_to_code` of UTF-8 and the parser's `PFETCH` read past a pattern that ends inside a character (upstream #16, #17, #19) | Oniguruma 2 to 5 |
| `parse` | an octal escape above 255 in a class writes outside the bit set (CVE-2017-9226) | Oniguruma 5 |
| `marshal`, `parse` | string compilation and case folding read past a piece of text that ends inside a character | Oniguruma 6, 8 |
| `regex` | `OP_PUSH_IF_PEEK_NEXT` and `OP_PUSH_OR_JUMP_EXACT1` read the subject before checking its end | Oniguruma 7 |

Each input, reduced, is under `crates/vkxp-ruby-sys/fuzz/fixed/`, and
`the_inputs_of_fixed_findings_no_longer_fail` of `tests/fuzz.rs` runs them in the normal
test step. Every one of them was run through targets built from the trees before the
fixes and gave its report there; that test and `tests/backports.rs` end with a
segmentation fault on those trees. One report was not explained; it is under "Follow-ups".

### The review

The network was reachable, and the sources the design names were consulted: the security
index of ruby-lang.org (2013-06 to 2026-08), the branch `ruby_1_8_7` after p374, the
`debian/patches` of `ruby1.8` in wheezy and squeeze-lts with the CVE list of Debian's
tracker, the source package `ruby-1.8.7.374-5.el6` of Red Hat, and a clone of
`kkos/oniguruma` with its whole history. Not reached: the errata pages of Red Hat (the
changelog of the package was used) and the page of `ruby1.8` in Debian's tracker (the
tracker's data was used). `docs/vendored-security.md` has 12 rows for built code of Ruby,
of which 2 are backported (patches 33, 34), 3 left and 7 do not apply; 45 rows for code
that is not built; and 58 rows for Oniguruma, of which 20 are backported (patches 1 to 8),
3 left and 35 do not apply (one of those, CVE-2019-19012, also names a related read that
is left). The backports of Oniguruma that no target had reached were
checked by the tests of `tests/backports.rs`; the three found by reading upstream's history
(9574bc1, 6d248fe, 1cb2c31) were reproduced first with the engine alone under
AddressSanitizer.
