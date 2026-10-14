# Local patches to Ruby 1.9.2-p0

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

An entry names one file. A change that touches two files is two entries that name each other.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

Patches 1 to 9 are the eight patches of the feasibility study of 2026-10-13, which built
this release with GCC 16 and clang 22 and ran its suites, also under `GC.stress`. Patches 1,
2, 3, 7, 8 and 9 repair the release for a compiler and a C library of today; the others make
the interpreter behave as vkxp embeds it. None changes what a script computes.

## 1. computed-goto-pointer

- File: `vm_exec.h`
- Patch: `patches/computed-goto-pointer.patch`
- Patched SHA-256: `b7c5e29f78bb7ce0fcc9f544d63afa2e791131b4bfa603df1b4efe764a0e6448`
- Reason: the release does not compile with GCC 16: `goto *GET_CURRENT_INSN()` jumps to
  an integer (`VALUE`), which is now an error ("computed goto must be pointer type"). The
  operand is cast to `void const *`.

## 2. cont-stack-copy-with-current-gcc

- File: `cont.c`
- Patch: `patches/cont-stack-copy-with-current-gcc.patch`
- Patched SHA-256: `649d6dcce4a988eeb15f692602b03f3d7702af0ef22165622c9cfc94b81f508e`
- Reason: three faults of the stack copying of Fibers and continuations at `-O1` and
  above. (1) `cont_restore_0` moves the stack pointer below the saved stack with `alloca`
  before the saved stack is copied back; the result of `alloca` was unused, the compiler
  removes the call, and the copy overwrites the frames that are doing the copying: every
  `Fiber.yield` crashed. The result is read, so the allocation stays. (2) `cont_capture` copies
  the machine stack to the heap and then calls `setjmp`; the compiler spills the local `cont`
  to its stack slot between the two, so the copy holds a stale slot, which is read after the
  second return of `setjmp`: calling a continuation crashed. The locals that are read after
  the second return are `volatile`, in `cont_capture` and in `fiber_store`, which has the same
  shape. (3) The jump buffer of a suspended Fiber is not marked, and glibc scrambles `rbp` in
  it, so a value held only in a callee-saved register was found by the collector only if some
  frame happened to push that register. `__builtin_unwind_init()` before the stack is
  captured pushes every callee-saved register into the frame that is copied.

## 3. zlib-crc-table-64bit

- File: `ext/zlib/zlib.c`
- Patch: `patches/zlib-crc-table-64bit.patch`
- Patched SHA-256: `2d2bd40f3d33267620c19734b29f54837696597fd4c378a17dd1dc08ed53e6c3`
- Reason: `Zlib.crc_table` read zlib's table of 32-bit values through
  `const unsigned long *`. Where `long` has 64 bits that returns wrong values and reads past
  the end of the table, and clang refuses the assignment. The pointer type becomes zlib's own
  `const z_crc_t *`, as patch 1 of the 1.8.7 log.

## 4. lazy-timer-thread

- File: `thread_pthread.c`
- Patch: `patches/lazy-timer-thread.patch`
- Patched SHA-256: `bbc3d12cd81578209e7535503cd9e19a15696a925eaa155399444799aa062569`
- Reason: the release starts a timer thread in `ruby_init`, a thread of the process that
  the host did not create and that wakes every 10 ms. With `ruby_vkxp_lazy_timer_thread` set
  before `ruby_init`, `rb_thread_create_timer_thread` creates no thread while the main thread
  is the only Ruby thread. A game without script threads then runs in a process with no thread
  beside the host's own, and a copy of the process made by `fork` lacks nothing. Patch 5 is
  the other half: it creates the thread when a script creates its first Ruby thread. Without
  the variable set the release's behaviour is unchanged.

## 5. lazy-timer-thread-create

- File: `thread.c`
- Patch: `patches/lazy-timer-thread-create.patch`
- Patched SHA-256: `6bc2a2555bbc47137dccbde3f963b1e8843b6772216923390221e9acb62db2ea`
- Reason: the other half of patch 4: `thread_create_core` calls
  `rb_thread_create_timer_thread` before it starts the native thread of a new Ruby thread, so
  that the timer thread exists from the first `Thread.new` on. The call does nothing when
  the timer thread already runs.

## 6. fixed-hash-seed

- File: `random.c`
- Patch: `patches/fixed-hash-seed.patch`
- Patched SHA-256: `0b3288d99e234081ef0fe0b125ab43d1981737faa4ee6ec06e37f4b47f81f184`
- Reason: the release draws the seed of `String#hash`, `Float#hash`, `Array#hash` and
  the other hashes from `/dev/urandom`, the time, the process id and a stack address, so the
  values differ in every process. With `ruby_vkxp_hashseed_fixed` set before `ruby_init`, the
  seed is `ruby_vkxp_hashseed`. The original runtime's values differ in every process too, so
  no game can depend on them; vkxp fixes the seed so that a run is repeatable. The seed of
  `rand` is not touched.

## 7. gc-mark-callee-saved-registers

- File: `gc.c`
- Patch: `patches/gc-mark-callee-saved-registers.patch`
- Patched SHA-256: `d257c2c90cde8caa186b96c84c07661bbdd9dcefc6794420ef02528125f100e4`
- Reason: `mark_current_machine_context` relies on `setjmp` to put the registers where
  the stack scan finds them, and glibc scrambles `rbp` (and `rsp` and the program counter) in
  a jump buffer. A value held only in `rbp` was not marked. `__builtin_unwind_init()` puts
  every callee-saved register into the frame, above the stack end that is taken afterwards.
  The same hole in the stack copy of Fibers is closed by patch 2.

## 8. proc-call-with-block-gc-guard

- File: `proc.c`
- Patch: `patches/proc-call-with-block-gc-guard.patch`
- Patched SHA-256: `87b8fea09c144dc24adc562e402b956a3d25386ee7d96e67e424302f45a0e29a`
- Reason: a crash under `GC.stress` (`test_curry` of `test/ruby/test_proc.rb`, GCC 16
  at `-O2`). `proc_call` and `rb_proc_call_with_block` keep the Proc of a passed block only
  through a pointer into it (`&proc->block`); nothing refers to the object and it was
  collected during the call. The object is kept in a local that `RB_GC_GUARD` holds until
  the call has returned.

## 9. clone-method-gc-guard

- File: `class.c`
- Patch: `patches/clone-method-gc-guard.patch`
- Patched SHA-256: `f37052e6c55efa1506edd095d099170498ef94879f3572d55beb729e20b6355a`
- Reason: a wrong result under `GC.stress` (`test_public_class_method` of
  `test/ruby/test_module.rb`: a wrong string with clang, `SystemStackError` with GCC). In
  `clone_method` the cloned instruction sequence is held only through its data pointer while
  `rb_add_method` allocates, and the object was collected before the method referred to it.
  `RB_GC_GUARD` keeps the object until after the call.

## 10. static-encodings

- File: `encoding.c`
- Patch: `patches/static-encodings.patch`
- Patched SHA-256: `cc6c62548576ce0b8990cbcf41e94f74c9bb9216b90dfb59bbaba35dba2fd71d`
- Reason: the release loads an encoding that is not registered with
  `require "enc/<name>.so"`. vkxp links the encodings the original runtime has and loads no
  file. `load_encoding` normalises the name as before (lower case, every other character than
  a letter or digit an underscore) and asks `ruby_vkxp_static_encoding`, a function of the
  embedder (`crates/vkxp-ruby192-sys/encodings.c`), for the `Init_` function of that name; it
  calls the function, or the name stays unknown. An encoding is so registered at the first
  look-up of its name, as measured on the original runtime, whose `Encoding.list` has four
  entries at start-up although Shift_JIS, Windows-31J and ISO-8859-1 are usable. `require_enc`
  is no longer used and is removed. Patch 11 does the same for transcoders.

## 11. static-encodings-transcoders

- File: `transcode.c`
- Patch: `patches/static-encodings-transcoders.patch`
- Patched SHA-256: `ffb77ade6ecc607d26bfd020667cce31f29836d2f5418995f3eef71b1259d228`
- Reason: follows patch 10. `load_transcoder_entry` loaded a transcoder that is only
  declared by the name of its library with `require "enc/trans/<lib>.so"`. The transcoders
  vkxp links are registered at start-up by their `Init_` functions and `transdb` is not
  linked, so nothing is declared by name; such an entry now gives no transcoder and no file
  is looked for.

## 12. host-stack-limit

- File: `thread_pthread.c`
- Patch: `patches/host-stack-limit.patch`
- Patched SHA-256: `bbc3d12cd81578209e7535503cd9e19a15696a925eaa155399444799aa062569`
- Reason: applies after patch 4. `ruby_init_stack` takes the size of the machine stack
  from `RLIMIT_STACK`, which describes the main thread of the process. vkxp runs the
  interpreter on a thread it creates: with a limit larger than that thread's stack a deep
  recursion would overrun the stack before the interpreter notices, and the depth of a
  recursion would depend on the user's shell. When `ruby_vkxp_stack_size` is set before
  `ruby_init_stack`, it is the size of the machine stack. Without it the release's behaviour
  is unchanged.

## 13. host-stack-limit-check

- File: `gc.c`
- Patch: `patches/host-stack-limit-check.patch`
- Patched SHA-256: `d257c2c90cde8caa186b96c84c07661bbdd9dcefc6794420ef02528125f100e4`
- Reason: applies after patch 7, and belongs to patch 12. Where the platform has an
  alternate signal stack, `ruby_stack_check` of the release returns 0 without looking: the
  release waits for the segmentation fault of a stack overflow and raises `SystemStackError`
  from its signal handler. vkxp gives the disposition of SIGSEGV back to the host, so a
  recursion that is deep in the machine stack and shallow in Ruby's own stack (one through a
  host function that calls back into the script) ran off the thread's stack and ended the
  process; a test of the stack limit showed it. When `ruby_vkxp_stack_size` is set, the
  function compares the depth with the limit that patch 12 sets, and such a recursion raises
  `SystemStackError`. Without the variable the release's behaviour is unchanged.

## 14. thread-wakeup-signal-with-first-thread

- File: `thread_pthread.c`
- Patch: `patches/thread-wakeup-signal-with-first-thread.patch`
- Patched SHA-256: `bbc3d12cd81578209e7535503cd9e19a15696a925eaa155399444799aa062569`
- Reason: applies after patches 4 and 12, and belongs to patch 4. The interpreter wakes
  a Ruby thread that blocks by sending it SIGVTALRM (`ubf_select`), for which
  `Init_native_thread` installs an empty handler. vkxp puts the signal dispositions of the
  process back after `ruby_init`, and the default action of SIGVTALRM ends the process: the
  test of the signal dispositions showed the handler, and killing a sleeping thread would
  have ended the program. With `ruby_vkxp_lazy_timer_thread` set, the handler is installed
  again where the timer thread is created, which is when a script creates its first Ruby
  thread; while the main thread is the only Ruby thread the signal is never sent, and the
  process keeps the disposition the host gave it.

## 15. glob-join-path-size

- File: `dir.c`
- Patch: `patches/glob-join-path-size.patch`
- Patched SHA-256: `3ab4044ea6f2a46c29b9871dcca6fc902854b65a0c4fe7c81edd0553d86b1312`
- Reason: a C library built into the program with `_FORTIFY_SOURCE` (the default of the
  compiler of Ubuntu 24.04) ended the process with "buffer overflow detected" at the first
  `Dir.glob` that joins a directory and a name; `test_dir.rb`, `test_require.rb` and
  `test_system.rb` of the release's suite ended so. `join_path` allocates the two lengths,
  a separator and a terminator, copies the directory and the separator, and appended the
  name with `strlcat(buf+len, name, len2)`, where `len2` counts the separator that is
  already written: the size it states is one byte more than is left of the buffer. Of the
  two kinds this is an idiom the check misreads, not a write past the buffer: `strlcat`
  wrote the name and its terminator, which fit exactly, and the check compares the stated
  size with the object. The name and its terminator are now copied with `memcpy` of their
  true length, which is our own correction and not taken from a later release. The result
  of `join_path` is unchanged.
