# Design

## Context

See `proposal.md` for motivation and `docs/decisions.md` (section "Ruby") for the settled
choices. Facts established on 2026-10-08 on the development machine (Fedora, x86_64, gcc 16.2,
clang 22.1, no system Ruby), by building and running:

- The 38 C files that make up `libruby-static.a` are, at the top level of the release:
  `array bignum class compar dir dln enum enumerator error eval file gc hash inits io marshal
  math numeric object pack parse process prec random range re regex ruby signal sprintf st
  string struct time util variable version dmyext` (`.c`). `parse.c` and `lex.c` ship
  pre-generated; `lex.c` is included by `parse.c` and not compiled on its own. The only file
  `configure` contributes is `config.h`.
- Compiling exactly those files with
  `-O2 -std=gnu89 -fwrapv -fno-strict-aliasing -fPIC -DRUBY_EXPORT -D_GNU_SOURCE=1`, plus
  `ext/zlib/zlib.c` with `-DHAVE_ZLIB_H -DOS_CODE=OS_UNIX`, gives objects byte-identical to a
  `configure` and `make` build, and an interpreter that passes `test/ruby` (625 tests,
  1653893 assertions, no failure), `test/zlib` (45 tests) and `sample/test.rb` (912 checks).
- Without `-std=gnu89` the sources do not compile under today's default C dialect. Without
  `-fwrapv`, `-O2` breaks the overflow checks that promote to `Bignum` (`2**64` evaluates to 0)
  while the build still succeeds. Only Ruby's tests reveal it.
- clang needs three more flags: `-Wno-error=int-conversion`,
  `-Wno-error=incompatible-pointer-types`, `-Wno-error=incompatible-function-pointer-types`.
- The release contains the `Zlib` binding but not zlib itself.
- Linking needs `-lcrypt -lm` besides zlib.
- `ruby_init()` installs its own handlers for HUP, INT, QUIT, USR1, USR2, ALRM, TERM, BUS, SEGV
  and PIPE, replacing the flags and mask of any existing handler. Creating a second Ruby thread
  arms a process-wide virtual timer (SIGVTALRM).
- The interpreter checks for stack overflow only on every 256th call and was seen to run up to
  462 KiB past the configured limit before raising. One Ruby method level costs about 3.8 KB of
  C stack, one level through a block about 13.8 KB.
- `ruby_init_ext("zlib.so", Init_zlib)` after `ruby_init()` defines `Zlib` and makes a later
  `require 'zlib'` return false. `ruby_init_ext` and `ruby_set_stack_size` are exported but
  declared in no header.
- `rb_eval_string` reports the file as `(eval)`, counts lines from 0 and rewrites exception
  messages. Calling Ruby's own `eval(source, nil, file, line)` gives clean messages and
  backtraces.
- Ruby's `puts` is buffered by C stdio until exit when standard output is not a terminal.
- The interpreter exports general names such as `dtoa`, `st_*` and `re_set_syntax`; the last is
  also a glibc function.
- The checked `config.h` is specific to x86_64 Linux with glibc (type sizes, stack direction,
  glibc `FILE` internals).

The launcher of this change puts the release tarball, the verified `config.h`, and the scripts,
test hosts and patch from that check under `local/ruby/` in the working copy (ignored by git).

## Goals / Non-Goals

**Goals:**

- `cargo build` produces the interpreter with no step outside cargo and no system Ruby.
- Wrong build flags cannot go unnoticed.
- A small, documented way for host code to start the interpreter, run code on its thread, call
  into Ruby and be called back, which the binding crate can build on.

**Non-Goals:**

- The RGSS API, script loading, `$KCODE`, the policy that disables process-spawning functions.
  Those belong to the binding.
- Oniguruma. The stock engine is used until its own change.
- Other targets. The build refuses them clearly.
- A safe, complete Rust wrapper of the Ruby C API. Declarations are added as the binding needs
  them.

## Decisions

### What is vendored

`third_party/ruby-1.8.7/` holds a subset of the release, at its original relative paths:

- to build: the 38 C files, `lex.c`, the headers `defines.h dln.h env.h intern.h missing.h
  node.h regex.h re.h ruby.h rubyio.h rubysig.h st.h util.h version.h`, `ext/zlib/zlib.c`;
- to run Ruby's tests: `main.c`, `ext/fcntl/fcntl.c`, `ext/stringio/stringio.c`,
  `ext/purelib.rb`, `test/runner.rb`, `test/ruby/`, `test/zlib/test_zlib.rb`, `lib/test/`,
  `lib/{delegate,fileutils,generator,optparse,tempfile,timeout,tmpdir}.rb`, `sample/test.rb`;
- for provenance: `parse.y`, `keywords`;
- licence: `COPYING`, `COPYING.ja`, `GPL`, `LGPL`, `LEGAL`, `README`.

About 160 files and 3.4 MB instead of 2585 files and 19.8 MB. Next to them: `MANIFEST.sha256`
(upstream hash per vendored path, generated from the verified tarball), `PATCHES.md` (one entry
per local patch: file, reason, the patch file under `patches/`) and a short `README.vkxp.md`
stating origin, licence and how the subset was chosen. A test recomputes the hashes: an
unpatched file must match the manifest, and a patched file must match the patched hash that its
`PATCHES.md` entry records, while the manifest keeps the upstream hash for provenance.

Alternative considered: downloading the tarball in the build script. Rejected: builds must work
offline and the fork will accumulate patches that belong in version control.

### Build script

`build.rs` uses the `cc` crate:

- the fixed file list above, `opt_level(2)` regardless of profile, `-std=gnu89 -fwrapv
  -fno-strict-aliasing -fvisibility=hidden`, position-independent code, defines `RUBY_EXPORT`
  and `_GNU_SOURCE=1`, warnings silenced, and the three extra flags when the compiler is clang;
- include paths: the directory of the target's `config.h`, then the vendored tree;
- `config.h` checked in at `crates/vkxp-ruby-sys/config/x86_64-unknown-linux-gnu/config.h`,
  chosen by the `TARGET` variable; any other target panics with a message naming it;
- `ext/zlib/zlib.c` compiled the same way with its two defines and the include path that
  `libz-sys` (feature `static`, so zlib is built from source) exports;
- `cargo:rustc-link-lib` for `crypt` and `m`; `cargo:rerun-if-changed` for the vendored tree,
  the config directory and the script.

`-fvisibility=hidden` is the one deliberate difference from the reference build: it keeps the
interpreter's symbols out of the dynamic symbol table of the final executable, so that names
like `re_set_syntax` cannot be picked up by a shared library in place of glibc's. Static linking
inside the executable is unaffected. Because of it the objects are no longer byte-identical to
the reference build, which is why the test suites below are part of the gate.

### The crc_table patch

`ext/zlib/zlib.c` reads zlib's 32-bit CRC table through `const unsigned long *`, which on 64-bit
returns wrong values and reads past the table. The patch changes the pointer type to
`const z_crc_t *`. It is the first entry of `PATCHES.md`.

### FFI surface

`vkxp_ruby_sys::ffi` is written by hand: `VALUE` and `ID` as `usize`, the special constants,
`#[repr(C)]` layouts of `RBasic`, `RObject`, `RString`, `RArray`, `RFloat` and `RData`, the
fixnum and type helpers that are macros in C, the class and exception globals, and the
functions needed here and by the binding's first steps (definition, calling, conversion, strings,
arrays, exceptions, protection, GC roots, global and instance variables, Marshal). A test
compiles a small C program against the vendored headers that prints `sizeof` and `offsetof` for
those structs and compares them with the Rust side, so a layout mistake cannot go unnoticed.

Alternative considered: `bindgen`. Rejected: it needs libclang at build time, and the headers
are K&R-style declarations that give it little type information anyway.

### One interpreter thread

`Interpreter::start(options)` spawns a thread with a 16 MiB stack (configurable upwards) and on
it: records the signal dispositions listed in the spec, anchors the stack with
`ruby_init_stack`, calls `ruby_init`, sets the stack limit to the stack size minus 1 MiB of
headroom, registers `Zlib`, names the script, makes `$stdout` and `$stderr` synchronous, puts
the recorded dispositions back, then serves a queue of closures. `Interpreter::run(f)` sends a
closure and waits for its result, so every Ruby call happens on that thread. A process-wide
flag refuses a second interpreter. Dropping the handle finalises Ruby, disarms the virtual
timer and restores the dispositions again.

In the running game the whole guest is one long closure on this thread. SIGVTALRM is left to
Ruby; the crate documents that other threads should block it, which the executable arranges
later.

### Entering and leaving Ruby

Ruby raises with `longjmp`. Two rules, documented in the crate and followed by its helpers:

- Calls into Ruby that may raise go through `rb_protect`, so the jump ends inside C and Rust
  sees a status. `eval_protected(source, file, line)` is built on it and calls Ruby's `eval`
  method rather than `rb_eval_string`, for clean messages and backtraces.
- A host function called by Ruby runs its body inside `catch_unwind`, returning a `Result`.
  Only after the body has returned and every Rust value is dropped does the wrapper raise, with
  `rb_exc_raise`, either the requested Ruby exception or one built from a panic message. No
  frame with a pending destructor is ever jumped over.

Values held only in host heap memory are invisible to the conservative collector and must be
registered with `rb_gc_register_address`; values in locals on the interpreter thread's stack
are found by the stack scan.

### Ruby's suites under `cargo test`

An integration test builds a Ruby executable in the build output directory: `main.c`,
`ext/fcntl/fcntl.c`, `ext/stringio/stringio.c` and a three-line `Init_ext` compiled with the
same compiler and flags, linked against the two archives the build script produced (their
location exported with `cargo:rustc-env`). It then runs each `test/ruby/test_*.rb` file as its
own process, several at a time, plus `test/zlib` and `sample/test.rb`, with a scratch `HOME`
and `TMPDIR`, and sums the reported totals. Serially the core suite takes about 19 s, in
parallel about 6 s.

Alternative considered: running the suites inside the embedded interpreter. Rejected: they
shell out to `./ruby` and `./miniruby` and expect a normal process.

## Risks / Trade-offs

- [The conservative collector misses a value kept only in a register or in host heap memory] →
  The rooting rule above; a test that forces many collections; the suites run on every gate.
- [A compiler upgrade miscompiles 2003-era C] → The pinned flags and the three suites in the
  gate turn that into a test failure instead of a wrong game.
- [The development files of libcrypt are missing on a build machine] → Link error naming
  `crypt`; the README lists the prerequisite.
- [3.4 MB of third-party source in the repository] → Accepted; it is the fork the project
  maintains from now on.
- [Tests need a C compiler at test time] → The build already needs one.

## Open Questions

- Whether `String#crypt` should later use a bundled implementation to drop the libcrypt
  dependency. It does not affect anything built here.
