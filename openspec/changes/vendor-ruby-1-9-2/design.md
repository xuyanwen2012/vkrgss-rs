## Context

Decision 4 of `plan-vx-ace`: ruby-1.9.2-p0 is vendored under `third_party/ruby-1.9.2/` as
1.8.7 is, built by a sys crate of its own, `vkxp-ruby192-sys`, that offers the Rust API of
`vkxp-ruby-sys`; the collector stress is part of the gate for it from the start. Decisions
6, 7 and 8: only the encodings of the original are linked and listed; `String#hash` has a
fixed seed; the timer thread is not started before a script makes a thread.

The input is a study of 2026-10-13 outside the repository, read by a flow at
`../../ref/tools/ruby192-feasibility/` ("the study" below): `REPORT.md` (sections 1, 2, 3,
5, 6 and the work packages A and B of section 9 are this change), the release tarball
`ruby-1.9.2-p0.tar.gz`, `config.h.linux-x86_64-gnu89`, `patches/01` to `08`,
`scripts/build-direct.sh`, `scripts/run-tests.sh`, `scripts/run-gc-stress-all.sh`,
`scripts/fiber-frames.rb`, `scripts/fiber-frames-stress.rb`, `hosts/embed_fork.c`,
`hosts/extinit_test.c`, `hosts/rust_host.rs`, `hosts/testlib/`, `logs/`. The values of the
original runtime are in `../../ref/tools/oracle/measurements/ace-v0-REPORT.md`, section 8.

What the study established and this change relies on:

- The tarball (SHA-256 `8c0c4e26...285c`, 3432 files) is the release the original embeds.
  It ships every generated source the core needs. Licence: the Ruby licence or the GPL
  version 2, as for 1.8.7; no LGPL file is involved.
- 131 C files compiled one by one with `-O2 -std=gnu89 -fwrapv -fno-strict-aliasing -fPIC`
  and one header give a working static library in about 4 seconds. `-std=gnu89` is needed
  (2133 errors with the default of GCC 16); without `-fwrapv` the build succeeds and 22
  more tests fail. The flags `-fno-omit-frame-pointer -fvisibility=hidden` that
  `vkxp-ruby-sys` adds also work.
- Eight patches: 01 computed goto, 02 the stack copy of Fibers and continuations, 03
  `Zlib.crc_table`, 04 the lazy timer thread, 05 the fixed hash seed, 06 the callee-saved
  registers in the collector's scan, 07 and 08 two values lost under `GC.stress`.
- On that build `bootstraptest` passes 21 of 23 files; `test/ruby` runs 2486 tests with 11
  failures and 3 errors, all explained; under `GC.stress` 84 of 94 files pass, 7 fail only
  what fails without stress, 3 did not finish in 840 seconds.
- 190 of the 197 C items that `vkxp-ruby-sys` declares exist under the same name. The
  other seven are `ruby_errinfo`, `ruby_top_self`, `ruby_set_stack_size` and the four
  hooks that vkxp patched into 1.8.7.
- The two interpreters define 868 global symbols in common: they cannot be linked into one
  program.

How 1.8.7 is done is the model throughout: `third_party/ruby-1.8.7/` (`README.vkxp.md`,
`MANIFEST.sha256`, `PATCHES.md`, `patches/`), `crates/vkxp-ruby-sys/` (`build.rs`,
`config/`, `src/`, `tests/`), the capability `embedded-ruby`.

## Goals / Non-Goals

Goals:

- The vendored tree with manifest, patch log and licence files, and a test that holds it.
- `vkxp-ruby192-sys`: the hermetic build, the raw declarations, the safe layer of
  `vkxp-ruby-sys` under the same names, the start-up state measured on the original.
- The tests named in the proposal, with the stressed run of the upstream selection inside
  the workspace tests.
- A rule, checked with the other dependency rules, that keeps the two interpreters apart.

Non-goals:

- The binding over the new crate, the guest executable, any RGSS class, `Win32API`, `DL`,
  `load_data`, the script loader, the policy on functions that start processes.
- Any change to `vkxp-ruby-sys` or to `third_party/ruby-1.8.7/`. The one exception is one
  more test function in `crates/vkxp-ruby-sys/tests/manifest.rs` (decision 2).
- aarch64. The crate is for `x86_64-unknown-linux-gnu` only (decision 4).
- Behaviour of the original that the two reports do not hold. In particular: whether
  `Encoding.list` grows after the first use of Shift_JIS; the six names of
  `Encoding.name_list`; the aliases; the order of `Encoding.list`; which further encoding
  names the original converts to or from.
- The start-up symbol table of the original (2170 names). A dump of 1.9 does not depend on
  it; only `Symbol#object_id` would show it.
- The run under AddressSanitizer, the fuzz targets and the review of later security fixes
  that `embedded-ruby` has for 1.8.7.
- Save states with a live script thread, and stopping or restarting the timer thread around
  `fork` (they belong to the guest role).
- A fix for the starvation of the main thread by a busy script thread
  (`test_list(TestThread)`).

## Decisions

1. **The vendored subset.** Paths are those of the release. Four groups, as the
   `README.vkxp.md` of 1.8.7 names them:
   - *To build the interpreter*: the C files of the build list (decision 3), every file
     they include from the tree (headers, `.inc`, `.def`-generated files, the C files that
     others include such as `vm_exec.c`, `vm_eval.c`, `vm_insnhelper.c`, `vm_method.c`,
     `eval_error.c`, `eval_jump.c`, `thread_pthread.c`, `id.c`, `lex.c`, `vsnprintf.c`),
     `include/`, `ext/zlib/zlib.c`. The list is taken from the compiler: the files under
     the tree that the dependency output (`-MD`) of the build names, and nothing else.
   - *To run Ruby's own tests*: `main.c`; of `ext/` the extensions the selected tests
     require (`stringio`, `fcntl`, `etc`, `continuation`, `fiber`, and `coverage` only if a
     selected test requires it); `test/runner.rb`, `test/ruby/`, `bootstraptest/`; of
     `lib/` the files the runs load (`test/unit`, `minitest` and what they and the selected
     tests require), found from `$LOADED_FEATURES` at the end of the runs. None of these is
     part of the library.
   - *For provenance*: `parse.y`, `defs/keywords`, `insns.def`.
   - *Licence*: `COPYING`, `COPYING.ja`, `GPL`, `LEGAL`, `README`.
   Added by vkxp and not part of the release: `MANIFEST.sha256`, `PATCHES.md`,
   `README.vkxp.md`, `patches/`. The prelude sources, `enc/encdb.c`, `enc/trans/transdb.c`,
   every encoding and transcoder not named in decision 5, `missing/`, the rest of `ext/`
   and `lib/`, documentation, `configure` and the other ports are left out.
   `README.vkxp.md` states origin, hash, licence, the parts with terms of their own that
   are vendored (the table of section 1 of the study, without the rows that are not
   vendored) and how the subset was chosen.

2. **Manifest and patch log.** `MANIFEST.sha256` and `PATCHES.md` have the format of
   1.8.7, because the same test reads them: `crates/vkxp-ruby-sys/tests/manifest.rs`
   gains one test function that calls its `check_tree` with `ruby-1.9.2`. That test does
   not link an interpreter, so it also runs where `vkxp-ruby192-sys` is empty. An entry of
   the patch log names one file; a patch of the study that touches two files (04:
   `thread_pthread.c` and `thread.c`) becomes two entries with two diffs that name each
   other. The diffs are those of the study with the comments that say "feasibility study"
   reworded to say what the code is for. The log starts with the eight patches under
   these names: `computed-goto-pointer`, `cont-stack-copy-with-current-gcc`,
   `zlib-crc-table-64bit`, `lazy-timer-thread`, `fixed-hash-seed`,
   `gc-mark-callee-saved-registers`, `proc-call-with-block-gc-guard`,
   `clone-method-gc-guard`; each reason is the fault and the fix of the table "What broke
   and how it is fixed" of the study, in the words of a log entry. Two more follow:
   `static-encodings` (decision 5) and `host-stack-limit` (decision 7). Further patches are
   written only for a fault that a test of this change shows (a value lost under stress, a
   test that fails for a reason not allowed by decision 10).

3. **The build.** `crates/vkxp-ruby192-sys/build.rs`, modelled on that of
   `vkxp-ruby-sys`: `cc`, a fixed file list, `-std=gnu89 -fwrapv -fno-strict-aliasing
   -fvisibility=hidden`, optimisation level 2, position-independent code, frame pointers
   forced, `RUBY_EXPORT` and `_GNU_SOURCE` defined, the three `-Wno-error` flags for clang.
   The file list is that of `scripts/build-direct.sh`: its `COMMON` list, the built-in
   encodings `ascii us_ascii unicode utf_8`, `enc/trans/newline`, `encoding`, `version`,
   `miniprelude`, `dmyext`, and `dmydln` in place of `dln` (nothing is loaded
   dynamically); then the encodings and transcoders of decision 5, the encodings with
   `-DONIG_ENC_REGISTER=rb_enc_register`; then `ext/zlib/zlib.c` against the zlib of
   `libz-sys` with `HAVE_ZLIB_H` and `OS_CODE=OS_UNIX`. `miniprelude.c` is compiled because
   `ruby.c` refers to `Init_prelude`; the crate never calls it. The header is the study's
   `config.h.linux-x86_64-gnu89`, checked in as
   `crates/vkxp-ruby192-sys/config/x86_64-unknown-linux-gnu/ruby/config.h` (it defines no
   `STACK_END_ADDRESS`, names no real `RUBY_EXEC_PREFIX`, and `RUBY_PLATFORM` is
   `x86_64-linux`). The package has `links = "vkxp_ruby192"`. Libraries: `pthread`, `rt`,
   `crypt`, `m`; libcrypt is linked dynamically, and the static switch of 1.8.7 comes with
   the change that puts this interpreter into a release package. The build script passes
   compiler, flags and directories to the tests as `VKXP_RUBY192_*` variables, as 1.8.7
   does.

4. **Other targets.** The build script compiles nothing unless the target is
   `x86_64-unknown-linux-gnu`; it then prints `cargo:warning` with the target's name. The
   crate's modules and every integration test are under
   `cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))`, and
   `libz-sys` is a dependency for that configuration only, so that a build for another
   target needs no C compiler for it. `cargo test --workspace` on the CI job
   `gate-aarch64` therefore keeps building and shows what it showed before. This differs
   from 1.8.7, which is built on aarch64 with the header of x86_64: for 1.9.2 nothing was
   tried there, and an interpreter with native threads and copied Fiber stacks is not
   offered on a guess.

5. **Encodings.** Linked: `enc/utf_16le.c`, `enc/shift_jis.c` (which carries
   Windows-31J), `enc/iso_8859_1.c`, and the transcoders `enc/trans/japanese_sjis.c`,
   `enc/trans/single_byte.c`, `enc/trans/utf_16_32.c`, beside the four built into the core
   and `newline`. `Init_encdb` and `Init_transdb` are neither vendored nor called, so no
   name of the release is declared.
   - At start-up the crate calls `Init_utf_16le` and the `Init_` functions of the three
     transcoder libraries. `Encoding.list` then holds ASCII-8BIT, UTF-8, US-ASCII and
     UTF-16LE.
   - Shift_JIS, Windows-31J and ISO-8859-1 are registered at the first look-up of their
     name. Patch `static-encodings`: `load_encoding()` in `encoding.c` no longer requires
     `enc/<name>.so`; it asks a function of the embedder for the `Init_` function of the
     normalised name and calls it. `load_transcoder_entry()` in `transcode.c` no longer
     requires a file either; without `transdb` it is not reached for a library. The table
     of the crate, a C file of its own beside the build script, has three entries:
     `shift_jis` and `windows_31j` (both `Init_shift_jis`) and `iso_8859_1`.
   - An unknown name stays unknown: `"あ".encode("EUC-JP")` raises, as measured.
   What the original does beyond the two measurements (four encodings listed; Shift_JIS,
   Windows-31J and ISO-8859-1 work; EUC-JP, GBK and Big5 raise) is the study's inference,
   not a measurement. Whether the list grows after first use, the aliases (`SJIS`, `CP932`,
   `BINARY`, which the original does not know), the order of the list and the alias
   `filesystem` are to be measured later and are not requirements here; see Follow-ups.

6. **The Rust API.** The source of `vkxp-ruby-sys/src` is the starting point; the files
   keep their names (`ffi.rs`, `interp.rs`, `call.rs`, `host.rs`, `protect.rs`, `root.rs`,
   `signals.rs`). The public items keep name and meaning, with these differences, which the
   crate documentation lists:

   | `vkxp-ruby-sys` | `vkxp-ruby192-sys` |
   |---|---|
   | `ffi::ruby_errinfo` (a variable) | `ffi::rb_errinfo()`, `ffi::rb_set_errinfo()`; `protect` and `Ruby::set_current_error` use them |
   | `ffi::ruby_top_self` (a variable) | `ffi::rb_vm_top_self()`, and `Ruby::top_self()` |
   | `ffi::ruby_set_stack_size` | absent; `Options::stack_size` is applied through patch `host-stack-limit` |
   | `ffi::ruby_vkxp_gc_stress`, `ffi::ruby_vkxp_gc_count` | absent; `GC.stress` and `GC.count` of the release |
   | `ffi::ruby_vkxp_select_symbols`, `ffi::ruby_vkxp_mark_game_scripts`, `SymbolTable`, `Options::symbols`, `Ruby::mark_game_scripts` | absent: symbols are numbered as the release numbers them |
   | `ffi::RString`, `ffi::RArray`, `ffi::rstring()`, `ffi::rarray()` | `ffi::rstring_ptr()`, `ffi::rstring_len()`, `ffi::rarray_ptr()`, `ffi::rarray_len()`, which follow `RSTRING_PTR` and its siblings for the embedded and the heap form |
   | `T_*`, flag bits | the values of the 1.9.2 headers |
   | `Ruby::str_new` | the same, makes ASCII-8BIT; new beside it `Ruby::str_new_utf8` for text |
   | `Ruby::eval`, `Ruby::eval_protected` | the same; the source is handed over as a UTF-8 string |
   | `SIGNALS_TAKEN_BY_RUBY` | the signals `Init_signal` of 1.9.2 takes |

   Everything else of `ffi` is declared under the name it has in 1.8.7, with the signature
   of the 1.9.2 headers, plus what the new items need (`rb_enc_str_new`,
   `rb_utf8_encoding`, `rb_enc_set_default_external`, the `Init_` functions, the variables
   of patches 04, 05 and `host-stack-limit`). The study compared the 190 common names, not
   their signatures: the test of decision 9 compares the signatures. How one binding
   source asks for what differs (`top_self`, the accessors, text against bytes) from both
   back ends is for the change that compiles the binding twice; `vkxp-ruby-sys` is not
   changed here.

7. **Start-up**, on the thread the crate creates, in this order, as `hosts/embed_fork.c`
   does: record the signal dispositions and the thread's signal mask; set the variables of
   patches 04 (lazy timer thread: on), 05 (hash seed: the constant `HASH_SEED`, which is 1,
   the value the study ran with) and `host-stack-limit` (the thread's stack size minus
   `STACK_HEADROOM`); `ruby_init_stack` with a local of the entry function; `ruby_init`;
   the static encodings and transcoders of decision 5; `Init_zlib()` called directly, not
   through `ruby_init_ext`, so that `zlib.so` is not a loaded feature; default external
   encoding UTF-8; `"."` pushed to the empty load path; `ruby_script`; standard output and
   error synchronous; `GC.stress = true` if `VKXP_GC_STRESS` is a number of 1 or more
   (unset, empty or 0: nothing; anything else is the error it is in `vkxp-ruby-sys`);
   dispositions and mask put back. `ruby_options` and `Init_prelude` are never called.
   Patch `host-stack-limit`: `ruby_init_stack()` in `thread_pthread.c` takes the size from
   a variable the host sets when it is not 0, instead of from `RLIMIT_STACK`. Without it a
   process whose limit is larger than the thread's stack would overrun the stack before
   the interpreter notices, and the depth of a recursion would depend on the user's shell.
   `GC.stress` has no "every n-th" form in this crate: the variable is on or off.

8. **One interpreter per program.** Three things keep it true. The two sys crates have
   different `links` keys and no crate depends on both: the tests of `vkxp-ruby192-sys`
   are test binaries of that package, and `xtask` depends on neither. `xtask/src/deps.rs`
   gets a fourth rule over the metadata it already reads: no workspace package reaches both
   `vkxp-ruby-sys` and `vkxp-ruby192-sys`, with development dependencies counted for this
   rule (a test binary links them); `vkxp-ruby192-sys` joins `GUEST_CRATES`. The rule runs
   with the others in the tests of `xtask`, so the gate runs it. The lists of crates in the
   capability `workspace-structure` are not edited by this change; the requirement "The
   two interpreters are never linked together" carries the rule (Follow-ups).

9. **Tests of the crate**, each file a test binary of `vkxp-ruby192-sys`, one interpreter
   per process as for 1.8.7:
   - `layout`: a C program compiled against the vendored headers prints sizes, offsets,
     tags and constants; a second C file declares every function of `ffi` with the types
     `ffi` gives it and must compile against the headers.
   - `symbols`: as for 1.8.7, with the prefixes `onig` and `Onig` added.
   - `interpreter`, `encodings`, `protect`, `calling`, `classes`, `signals_*`,
     `timer_thread`, `hash_seed` (starts itself a second time as a child process),
     `stack_limit`, `gc_stress_on`, `gc_stress_off`: the scenarios of the specification.
   - `original`: identity, `Marshal`, `srand` and `rand`, float text, with the values of
     the specification, which are those of `scripts/compare-with-original.rb` where the
     study found them equal.
   - `fork_frames`, `fork_frames_stressed`: `scripts/fiber-frames.rb` and
     `fiber-frames-stress.rb` adapted; the host function forks with `libc::fork`; a copy
     runs the script to its end inside the closure, writes its digest to a pipe and leaves
     with `_exit`; the original collects the digests and the exit statuses.
   - `fiber_host_frames`: `hosts/rust_host.rs` as a test.
   - `upstream`, `upstream_stress`: decision 10.

10. **The upstream suites.** A test executable `ruby` is linked from the archives of the
    build script, `main.c`, the test-only extensions and an `Init_ext` that registers them
    and the static encodings of decision 5 (after `hosts/extinit_test.c`, without the
    encodings that are not linked). It runs `ruby_options`, so the prelude
    (`miniprelude.c`) runs in it; that is the test executable, not the library. The
    `rbconfig.rb` that the test libraries read is not part of the release: a short one
    written for the purpose lives in `crates/vkxp-ruby192-sys/tests/testlib/`, with
    `test_prelude.rb` and `gc_stress.rb` of the study adapted; the study's own
    `rbconfig.rb` holds paths of the study's machine and is not copied.
    - `bootstraptest`: every file, each under a timeout, with that executable as runner and
      as target. Left out by name: `test_fork.rb` and `test_thread.rb`, where `fork` from a
      Ruby thread that is not the main one hangs the child in `pthread_cond_destroy` with
      the C library of today.
    - `test/ruby`: one process per file, several at a time. The list
      `tests/upstream.allow` names what is left out and what may fail, each entry with its
      reason, in the format of `xtask/src/stress.skip` (comment lines as the reason, then
      lines of `ruby/test_x.rb` or `ruby/test_x.rb#test_y`). Allowed reasons, and no other:
      (a) the 14 failures the study explains (five of `TestTimeTZ`, `test_str_crypt`, two
      of `TestNotImplement`, two of `TestRequire` and `test_require` of `TestRubyOptions`,
      `test_local_barrier`, `test_interrupt` of `TestBignum`, `test_list` of `TestThread`;
      the names are in `logs/test-ruby-final4-O2.log`); (b) a test that names an encoding,
      a transcoder or a loadable extension that is not linked, with that name in the
      reason; a file whose tests are mostly of that kind (`test_m17n.rb`,
      `test_m17n_comb.rb`, `test_transcode.rb`, `test_econv.rb`, `test_io_m17n.rb`,
      `test_encoding.rb` are the candidates) is left out whole. A test on the list may
      pass: the time zone data and the C library differ between machines. A failure with
      another cause is a fault of the build and is fixed, by a recorded patch if it is in
      the vendored code.
    - The harness gives the children the default disposition of SIGINT and a search path
      of its own, as the harness of 1.8.7 does, and fails when a file reports no test.
    The study ran the suites with all 95 encodings linked and `bootstraptest` with
    `miniruby` as target; the counts it reports (2486 tests, 879 assertions) are therefore
    not expected here and are not in the specification.

11. **Collector stress.** The stressed run loads `gc_stress.rb` (`GC.stress = true`)
    before each file, one process per file, with the same list of allowed failures.
    - In the workspace tests (`upstream_stress`, not ignored, so the gate runs it): the
      selected files whose stressed run took under 60 seconds in
      `logs/gc-stress-all-final4-O2.txt`. In the study these 83 files took 1240 seconds of
      wall time in sum with ten at a time, a few minutes on the development machine.
    - `cargo xtask stress` gets a step `suites-1.9.2` that runs the ignored test of the
      same file for the slow ones: `test_array.rb`, `test_assignment.rb`, `test_fiber.rb`,
      `test_literal.rb`, `test_process.rb`, `test_sprintf_comb.rb`, `test_system.rb` (and
      `test_io_m17n.rb` if it is selected). It needs no sanitizer.
    - `xtask/src/stress.skip` names `test_integer_comb.rb`, `test_m17n_comb.rb` and
      `test_yield.rb` for that step: they did not finish in 14 minutes under stress in the
      study and are unexamined there.
    - `test_io.rb` reported "0 tests" under stress in the study's log and was counted as
      passing. The harness treats that as a failure. The cause is to be found; if it is
      not found within this change, the file is left out of the stressed runs by name with
      exactly that statement as the reason, and a line under Follow-ups says so.
    The steps of the gate do not change: the test `the_stress_run_is_not_a_step` of
    `xtask/src/gate.rs` stays as it is, because the stressed selection is an ordinary test
    of the workspace and neither `cargo xtask stress` nor `VKXP_GC_STRESS` enters a step.
    Fibers under stress are in the gate through `fork_frames_stressed` and
    `fiber_host_frames`; `test_fiber.rb` under stress (440 seconds in the study) is not.

12. **Licences.** `THIRD-PARTY.md` gets a section "Ruby 1.9.2-p0" after that of 1.8.7:
    what the directory holds, the terms (Ruby licence or GPL version 2, the parts of
    `LEGAL`, Oniguruma built in under its BSD terms, no LGPL file), and the three files
    `README.vkxp.md`, `MANIFEST.sha256`, `PATCHES.md`. `deny.toml` does not change: no Rust
    dependency is added. The release package does not change: `vkxp` does not link this
    interpreter yet.

## Implementation notes

- A flow copies from `../../ref/tools/ruby192-feasibility/` only: files of the tarball
  that belong to the subset, the configuration header, the eight patches, and the scripts
  and hosts named above as the starting point of tests. Nothing of `ref/` or `games/` is
  committed otherwise; no Wine and no original runtime is run. Scratch output goes under
  `target/` or `local/` of the clone.
- To find the subset: build once with the study's `scripts/build-direct.sh` under
  `target/`, or with the new build script and `-MD`, and collect the paths under the tree.
- Under gnu89 a call to an undeclared C function compiles and returns `int`. Every C file
  this change writes includes the header of what it calls (`ruby/encoding.h` for
  `rb_utf8_encoding`).
- `Init_shift_jis` must be safe to reach from either of its two names; after the first
  call both names are registered and the table is not asked again.
- A copy made by `fork` never calls `rb_thread_atfork()`: it reseeds `rand`.
- `ruby_init` blocks every signal on the interpreter thread; `signals.rs` restores the
  mask as well as the dispositions.
- `AGENTS.md` names the crates that may hold `unsafe`; `vkxp-ruby192-sys` joins them.
  `README.md` has a table of crates; `docs/building.md` describes the interpreter build and
  the targets; `docs/development.md` the stress command and its steps.
- `Zlib::ZLIB_VERSION` is that of the zlib `libz-sys` builds, not the 1.2.3 of the
  original; the same holds for 1.8.7 today.
- Known differences from the original that this change does not remove and no test may
  pin: `RUBY_PLATFORM`; `2**30` is a `Fixnum` and `1.size` is 8; `"%.17g" % (10.0**23)`
  (the `pow` of the C library); hash values are 64-bit.

## Follow-ups

- Measure on the original (probe `scripts/ace-probe-interpreter.rb` of the study):
  `Encoding.list` after the first use of Shift_JIS, the six names of `Encoding.name_list`,
  `Encoding.aliases`, the order of the list, `Encoding.find("filesystem")` (ASCII-8BIT
  there) and `"locale"` (raises there), which further names convert, and `Encoding.constants`.
  Then the change that matches them.
- Found while implementing decision 5: in the release the aliases (`BINARY`, `CP932`, `SJIS`,
  `ASCII`), the replicas (Windows-31J of Shift_JIS, Windows-1252 of ISO-8859-1) and the
  constants of the three encodings of the core (`Encoding::ASCII_8BIT`, `Encoding::UTF_8`,
  `Encoding::US_ASCII`) are all declared by `encdb.h`, which is not linked. So, differently
  from what decision 5 and the paragraph above it assume: (1) `enc/shift_jis.c` registers
  Shift_JIS alone, and the table's entry for `shift_jis` and `windows_31j` is a function of
  `encodings.c` that calls `Init_shift_jis` and then `rb_enc_replicate("Windows-31J", ...)`,
  not `Init_shift_jis` itself; (2) vkxp knows no alias at all, `BINARY` included, as on the
  original; (3) `Encoding::UTF_8`, `Encoding::ASCII_8BIT` and `Encoding::US_ASCII` are
  undefined in the library (`Encoding::UTF_16LE` is defined, and `Encoding::Shift_JIS`,
  `Encoding::Windows_31J` and `Encoding::ISO_8859_1` once their names were looked up).
  Whether the original defines these constants is not in the two reports; if it does, the
  crate declares them at start-up (`rb_encdb_declare`). The test executable of decision 10
  declares the three constants in its `Init_ext` and looks the three lazy names up in
  `test_prelude.rb`, because the suites name encodings by constant throughout.
- The capability `workspace-structure` lists the crates of the workspace and of the guest
  side by name; `vkxp-ruby192-sys` and the binding package over it are to be added there
  by the change that adds that package.
- The run under AddressSanitizer for 1.9.2 (the stack copying of Fibers needs what patches
  18 and 19 of the 1.8.7 log did), fuzz targets, and the review of security fixes later
  than 1.9.2-p0.
- An "every n-th allocation" form of the stress switch, if the stressed run of the binding
  is too slow without it.
- aarch64: a verified header and a first run of the suites.
- `test_integer_comb.rb`, `test_m17n_comb.rb` and `test_yield.rb` under stress with a
  longer limit.
- The static link of libcrypt, when a release package holds this interpreter.
