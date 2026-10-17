# Local patches to Ruby 1.9.2-p0

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkrgss-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

An entry names one file. A change that touches two files is two entries that name each other.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

Patches 1 to 9 are the eight patches of the feasibility study of 2026-10-13, which built
this release with GCC 16 and clang 22 and ran its suites, also under `GC.stress`. Patches 1,
2, 3, 7, 8 and 9 repair the release for a compiler and a C library of today; the others make
the interpreter behave as vkrgss embeds it. None changes what a script computes.

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
- Patched SHA-256: `652deac0ea357f58e9b7d5b5a6f015abd910a44c4460bacebcaf868cc780e1ff`
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
- Patched SHA-256: `6271416d725e49d9f5127b52f55771019b6730f916698cea38073d369e80acea`
- Reason: `Zlib.crc_table` read zlib's table of 32-bit values through
  `const unsigned long *`. Where `long` has 64 bits that returns wrong values and reads past
  the end of the table, and clang refuses the assignment. The pointer type becomes zlib's own
  `const z_crc_t *`, as patch 1 of the 1.8.7 log.

## 4. lazy-timer-thread

- File: `thread_pthread.c`
- Patch: `patches/lazy-timer-thread.patch`
- Patched SHA-256: `d145c6e75822737d96b0956b79862232b88857ca86e6d89937bda1ba8f58bdf0`
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
- Patched SHA-256: `1606cf65a1ddd5e8560da9774299da93c5a4a1d50bb311cdf92823d92af960bf`
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
  no game can depend on them; vkrgss fixes the seed so that a run is repeatable. The seed of
  `rand` is not touched.

## 7. gc-mark-callee-saved-registers

- File: `gc.c`
- Patch: `patches/gc-mark-callee-saved-registers.patch`
- Patched SHA-256: `8c2629cfc340733c01dc1cdda5e4f19e143f262a7fe28b057cfd21e94db24de7`
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
  `require "enc/<name>.so"`. vkrgss links the encodings the original runtime has and loads no
  file. `load_encoding` normalises the name as before (lower case, every other character than
  a letter or digit an underscore) and asks `ruby_vkxp_static_encoding`, a function of the
  embedder (`crates/vkrgss-ruby192-sys/encodings.c`), for the `Init_` function of that name; it
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
  vkrgss links are registered at start-up by their `Init_` functions and `transdb` is not
  linked, so nothing is declared by name; such an entry now gives no transcoder and no file
  is looked for.

## 12. host-stack-limit

- File: `thread_pthread.c`
- Patch: `patches/host-stack-limit.patch`
- Patched SHA-256: `d145c6e75822737d96b0956b79862232b88857ca86e6d89937bda1ba8f58bdf0`
- Reason: applies after patch 4. `ruby_init_stack` takes the size of the machine stack
  from `RLIMIT_STACK`, which describes the main thread of the process. vkrgss runs the
  interpreter on a thread it creates: with a limit larger than that thread's stack a deep
  recursion would overrun the stack before the interpreter notices, and the depth of a
  recursion would depend on the user's shell. When `ruby_vkxp_stack_size` is set before
  `ruby_init_stack`, it is the size of the machine stack. Without it the release's behaviour
  is unchanged.

## 13. host-stack-limit-check

- File: `gc.c`
- Patch: `patches/host-stack-limit-check.patch`
- Patched SHA-256: `8c2629cfc340733c01dc1cdda5e4f19e143f262a7fe28b057cfd21e94db24de7`
- Reason: applies after patch 7, and belongs to patch 12. Where the platform has an
  alternate signal stack, `ruby_stack_check` of the release returns 0 without looking: the
  release waits for the segmentation fault of a stack overflow and raises `SystemStackError`
  from its signal handler. vkrgss gives the disposition of SIGSEGV back to the host, so a
  recursion that is deep in the machine stack and shallow in Ruby's own stack (one through a
  host function that calls back into the script) ran off the thread's stack and ended the
  process; a test of the stack limit showed it. When `ruby_vkxp_stack_size` is set, the
  function compares the depth with the limit that patch 12 sets, and such a recursion raises
  `SystemStackError`. Without the variable the release's behaviour is unchanged.

## 14. thread-wakeup-signal-with-first-thread

- File: `thread_pthread.c`
- Patch: `patches/thread-wakeup-signal-with-first-thread.patch`
- Patched SHA-256: `d145c6e75822737d96b0956b79862232b88857ca86e6d89937bda1ba8f58bdf0`
- Reason: applies after patches 4 and 12, and belongs to patch 4. The interpreter wakes
  a Ruby thread that blocks by sending it SIGVTALRM (`ubf_select`), for which
  `Init_native_thread` installs an empty handler. vkrgss puts the signal dispositions of the
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

## 16. timer-thread-in-a-copy

- File: `thread_pthread.c`
- Patch: `patches/timer-thread-in-a-copy.patch`
- Patched SHA-256: `d145c6e75822737d96b0956b79862232b88857ca86e6d89937bda1ba8f58bdf0`
- Reason: applies after patches 4, 12 and 14; it is numbered after 15, which touches another file. A copy of the process made by `fork` has no
  timer thread, and `rb_thread_reset_timer_thread`, which the interpreter calls in its own
  child and vkrgss in a copy that is a state, only forgets the thread. The lock and the
  condition variable of the timer thread stay as that thread left them in the copied
  process, which waits on the condition variable or holds the lock at any moment. The timer
  thread the copy starts for its next script thread then left its loop at once, at a wakeup
  that was not a timeout, and a busy main thread of the copy never gave way to the new
  thread: a test of a copy of the process showed it once in a run of the workspace tests
  under load. `native_reset_timer_thread` makes both afresh. After the timer thread was
  stopped and joined, its other caller, they are free and nothing changes.

## 17. gvl-handover

- File: `thread.c`
- Patch: `patches/gvl-handover.patch`
- Patched SHA-256: `1606cf65a1ddd5e8560da9774299da93c5a4a1d50bb311cdf92823d92af960bf`
- Reason: applies after patch 5. The global VM lock of the release is a plain mutex, which
  is not fair: a thread that gave it up at the end of its time slice (`rb_thread_schedule`)
  took it again before the thread that waited for it had woken, so a main thread that never
  blocks kept a new thread from running. Of twenty new threads in a row, each awaited by a
  main thread in a loop without blocking, seven had not run after 30 seconds and the others
  ran after 0.1 to 28 seconds, in the original process and in a copy made by `fork` alike;
  the timer thread had set the interrupt of the main thread every 10 ms and the main thread
  had given way every time. A thread that takes the lock now counts itself as waiting
  (`gvl_acquire`), and `gvl_yield` sleeps, when a thread waits, until another thread has
  taken the lock; each of the twenty threads then runs after one time slice. The count
  belongs to the threads of one process: `ruby_vkxp_gvl_atfork` forgets it in a copy,
  called by `rb_thread_atfork` and by vkrgss's `restart_thread_timer`.

## 18. strtod-long-digit-string

- File: `util.c`
- Patch: `patches/strtod-long-digit-string.patch`
- Patched SHA-256: `5907ec59e6834567f6152955daeb5374dba1172893bd63370a77823e502739f0`
- Reason: CVE-2013-4164, the fix patch 33 of the tree of Ruby 1.8.7 carries, which this
  tree lacked. `ruby_strtod` allocated its big numbers through `Balloc`, whose free list has
  `Kmax + 1` entries; a digit string long enough asked for a larger one, and `Balloc` and
  `Bfree` indexed the list past its end: a heap overflow that `Marshal.load` of a dumped
  Float reaches, as `String#to_f` does. Sizes above `Kmax` are now allocated and freed
  outside the list, and fraction digits after the first `DBL_DIG * 4` are skipped, which no
  double can tell apart. `tests/backports.rs` of `vkrgss-ruby192-sys` crashed with SIGSEGV
  before the patch.

## 19. asan-conservative-scan

- File: `gc.c`
- Patch: `patches/asan-conservative-scan.patch`
- Patched SHA-256: `8c2629cfc340733c01dc1cdda5e4f19e143f262a7fe28b057cfd21e94db24de7`
- Reason: applies after patches 7 and 13. For the sanitized run of the release's suites
  (`tests/sanitized.rs` of `vkrgss-ruby192-sys`), as patch 18 of the tree of Ruby 1.8.7. The
  collector is conservative: `mark_locations_array` reads every word of the machine stack
  and of the saved registers, the red zones that AddressSanitizer puts around local
  variables included, and the sanitizer stopped the first collection. The function carries
  the compiler's `no_sanitize_address` attribute. Without the sanitizer the attribute
  changes nothing.

## 20. asan-stack-copy

- File: `cont.c`
- Patch: `patches/asan-stack-copy.patch`
- Patched SHA-256: `652deac0ea357f58e9b7d5b5a6f015abd910a44c4460bacebcaf868cc780e1ff`
- Reason: applies after patch 2. For the same run, as patch 19 of the tree of Ruby 1.8.7. A
  Fiber switch and a continuation copy the machine stack to the heap in
  `cont_save_machine_stack` and back in `cont_restore_1`, red zones included, and the
  sanitizer's `memcpy` stopped `test_fiber.rb` and the other files that switch Fibers with
  a stack-buffer-underflow. Under AddressSanitizer the copied range is declared addressable
  before each copy (`__asan_unpoison_memory_region`); the frames that are live at a switch
  lose their red zones, so the run can miss a stack overflow in them and cannot report a
  false one. In any other build the macro is empty.

## 21. dtoa-terminator-room

- File: `util.c`
- Patch: `patches/dtoa-terminator-room.patch`
- Patched SHA-256: `5907ec59e6834567f6152955daeb5374dba1172893bd63370a77823e502739f0`
- Reason: applies after patch 18. Found by the sanitized run, in `test_sprintf.rb`: a heap
  overflow of one byte. `nrv_alloc` copies a short string with its terminator into a
  buffer of `n` bytes, and its three callers in `BSD__hdtoa`, the conversion of `%a` and
  `%A`, pass the length without the terminator (`"Infinity"`, `"NaN"` and `"0"`), so the
  terminator was written past the buffer: `"%a" % 0.0` reaches it. The allocator's rounding
  hid it. The buffer now has room for the terminator, as the same function of the tree of
  Ruby 1.8.7 has; this is our own correction. `ruby_dtoa`, which the other conversions use,
  counts the terminator itself (`rv_strdup`) and gets one byte more than before. The text
  that is returned is unchanged.

## 22. time-load-month-range

- File: `time.c`
- Patch: `patches/time-load-month-range.patch`
- Patched SHA-256: `ec140cc52fd93263b8c48fbc9878efd90325cebbf3df5f90e5ec1fe1f160c61a`
- Reason: found by the fuzz target `marshal192` (change `fuzz-and-review-ruby-1-9-2`).
  `time_mload`, which `Marshal.load` calls for a dumped Time, takes the month from four bits,
  1 to 16, and `calc_tm_yday` indexed the tables of the twelve months with it: a read past
  their end. A month above 12 is now the error of a dump of another format, "marshaled time
  format differ" (`TypeError`), which the function raises for a wrong length. `Time#_dump`
  never writes such a month; what the original runtime makes of one (it reads past the same
  tables) was not measured. Input: `crates/vkrgss-ruby192-sys/fuzz/fixed/marshal/time-month-above-12.bin`.

## 23. parser-fuzz-findings

- File: `parse.y`
- Patch: `patches/parser-fuzz-findings.patch`
- Patched SHA-256: `dcd851d90cdde49d9a27c64ea688bfa76be187aa299b034337df17539f03753d`
- Reason: found by the fuzz target `parse192` (change `fuzz-and-review-ruby-1-9-2`). Two
  defects of the parser. The rule of a regexp literal joins the pieces of text after an
  interpolation; after the first piece the one it joins to is an element of the list, whose
  string is that of its head, and the rule took the element itself for the string, so that
  `/#$y #{"s"}#{"t"}/`, a valid script, read a null pointer. It takes the string of the
  head, as later releases do. And `assign_in_cond` looked at the type of the value of an
  assignment that is a null pointer after a syntax error in it (`!w=%w[`); it tests the
  pointer first, as patch 30 of the tree of 1.8.7 does. Inputs:
  `crates/vkrgss-ruby192-sys/fuzz/fixed/parse/{regexp-pieces-after-interpolation,condition-null-value}.rb`.

## 24. parser-fuzz-findings-generated

- File: `parse.c`
- Patch: `patches/parser-fuzz-findings-generated.patch`
- Patched SHA-256: `57cd6e133392ae4f77a05d634e29d6abba2675c99d37cc57a1716ca54ede240b`
- Reason: follows patch 23. The same lines of the pre-generated parser, changed by hand.

## 25. regexp-escape-incomplete-character

- File: `re.c`
- Patch: `patches/regexp-escape-incomplete-character.patch`
- Patched SHA-256: `68c224b0ec35949008496b71a924380014a533ffced099707ef4ad2654a27d9e`
- Reason: found by the fuzz target `regex192` (change `fuzz-and-review-ruby-1-9-2`).
  `unescape_nonascii` checks that a pattern is valid text before the engine sees it, but
  after a backslash it copied one byte without looking at the character: for a character
  of several bytes the next round found the rest invalid and gave "invalid multibyte
  character", unless the end cut the character off after its first byte, and then the
  engine read past the pattern. A character of several bytes after a backslash, whole or
  cut off by the end, now gives that error at once; a single byte that is no character
  is copied as before. Input: `crates/vkrgss-ruby192-sys/fuzz/fixed/regex/escape-incomplete-character.txt`.

## 26. class-wide-character-after-range

- File: `regparse.c`
- Patch: `patches/class-wide-character-after-range.patch`
- Patched SHA-256: `1684ac5ad07fea42e0d9ac21ac2e90c6f01204d977b8027eb0f50222c90e4734`
- Reason: found by the fuzz target `regex192` (change `fuzz-and-review-ruby-1-9-2`); the
  engine's rows of `docs/vendored-security.md` name the upstream fixes. In a class, a
  character after a range and a hyphen (`[0-9-ゟ]`) or after two hyphens was taken for one
  byte whatever its code, and `next_state_val` set the bit of the code in the bit set of 256
  bits: a write past it. The two places go to the code that tells a byte from a code point,
  as upstream 1ca27ef (#21), and `next_state_val` refuses a byte value above 255 with
  "invalid code point value", as upstream b4bf968 (CVE-2017-9226). Input:
  `crates/vkrgss-ruby192-sys/fuzz/fixed/regex/class-wide-character-after-range.txt`.

## 27. match-length-cycle

- File: `regcomp.c`
- Patch: `patches/match-length-cycle.patch`
- Patched SHA-256: `f9972fb4e00b250417e84ac5dfc304e5cfebb7041467ae0d3d3773f5a66e56e8`
- Reason: found by the fuzz target `regex192` (change `fuzz-and-review-ruby-1-9-2`).
  `get_min_match_length` and `get_max_match_length` follow a back-reference into its group,
  and groups that refer to each other, as in `()(\4)()(\2)`, led back without end: the stack
  was exhausted when the pattern was compiled. A back-reference that is being measured
  already is now taken as 0 for the shortest and as no limit for the longest length, which
  are bounds of any length, so the optimisation that uses them stays correct. This is our own
  correction; upstream rewrote both functions (7fe8c3f). Input:
  `crates/vkrgss-ruby192-sys/fuzz/fixed/regex/groups-referring-to-each-other.txt`.

## 28. colon3-name-not-marked

- File: `gc.c`
- Patch: `patches/colon3-name-not-marked.patch`
- Patched SHA-256: `8c2629cfc340733c01dc1cdda5e4f19e143f262a7fe28b057cfd21e94db24de7`
- Reason: applies after patches 7, 13 and 19; found by the fuzz target `parse192` (change
  `fuzz-and-review-ruby-1-9-2`). The collector marked the second field of a `NODE_COLON3`,
  the node of `::Name`, as a node, but it holds the name. The name of a constant has its
  lowest bit set and was skipped as a Fixnum; `class ::c`, a syntax error, leaves the name
  of a local variable there, and a collection while the node was alive read through it and
  ended the process. The node is marked without that field. Input:
  `crates/vkrgss-ruby192-sys/fuzz/fixed/parse/class-name-not-constant.rb`, which needs a
  collection at the right moment; `tests/fuzz.rs` compiles it under `GC.stress`.

## 29. pack-encodes-line-feed

- File: `pack.c`
- Patch: `patches/pack-encodes-line-feed.patch`
- Patched SHA-256: `d3a5bba42951dc5676a0fcbf9aa1b1cb699e5760af5d0df746ac61dcf8663747`
- Reason: backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). CVE-2014-4975. `encodes`, the
  conversion of `m` and `u`, fills a buffer of 4096 bytes in groups of four and empties it
  when less than a group is free, so a last group can fill it; the line feed after that
  group was written one byte past it, on the stack (`['a' * 3071].pack('m3072')`). The
  buffer has one byte more than the loop fills, as in upstream's fix. Its test in
  `tests/backports.rs` of `vkrgss-ruby192-sys` gave a report of AddressSanitizer in the
  sanitized interpreter built before the patch.

## 30. unpack-count-overflow

- File: `pack.c`
- Patch: `patches/unpack-count-overflow.patch`
- Patched SHA-256: `d3a5bba42951dc5676a0fcbf9aa1b1cb699e5760af5d0df746ac61dcf8663747`
- Reason: applies after patch 29; backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). CVE-2018-8778, as patch 34 of
  the tree of 1.8.7. A count of `String#unpack` above `LONG_MAX` became negative, which the
  checks of `@`, `X` and `x` let through, and the position moved before or past the string.
  Such a count is now the `RangeError` "pack length too big" that a count beyond `ULONG_MAX`
  gave already.

## 31. sprintf-non-finite-width

- File: `sprintf.c`
- Patch: `patches/sprintf-non-finite-width.patch`
- Patched SHA-256: `9a2e1424431af3d098056d263e177b707c048270ed5633bdc0801ca7c443020c`
- Reason: backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). CVE-2017-0898, fixed
  differently: `rb_str_format` formats infinity and NaN into `need + 1` bytes, and with the
  largest width that size wrapped to a negative number: the buffer did not grow, and the
  text was written as far as the width past it (`format('%2147483647f', 1.0 / 0)` ended the
  process). That width is now the `ArgumentError` "width too big", the message of a width
  too big in the format itself. Upstream rewrote the block. Every smaller width formats as
  before.

## 32. str-buf-cat-capacity

- File: `string.c`
- Patch: `patches/str-buf-cat-capacity.patch`
- Patched SHA-256: `c5a038e8e2de7785c3743a4b5a3de180222da50a2ad5faad46d054f2f162e48a`
- Reason: backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). CVE-2014-3916. When a string
  grows near `LONG_MAX / 2` bytes, `str_buf_cat` took the number of pages of the new length
  as the capacity instead of the length rounded up to a page, as upstream's fix corrects. On
  x86_64 the branch needs a string of 2^62 bytes, so no test shows it.

## 33. zlib-ungets-room

- File: `ext/zlib/zlib.c`
- Patch: `patches/zlib-ungets-room.patch`
- Patched SHA-256: `6271416d725e49d9f5127b52f55771019b6730f916698cea38073d369e80acea`
- Reason: applies after patch 3; backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). CVE-2026-27820.
  `zstream_buffer_ungets`, which `Zlib::GzipReader#ungetc` calls with a string, grew the
  buffer only when it was full and then wrote the bytes put back in front of the rest: past
  its end when less than their length was free. It grows the buffer when less than the
  length is free. Upstream's fix was not compared. Its test in `tests/backports.rs` gave a
  report of AddressSanitizer in the sanitized interpreter built before the patch.

## 34. engine-parser-end-reads

- File: `regparse.c`
- Patch: `patches/engine-parser-end-reads.patch`
- Patched SHA-256: `1684ac5ad07fea42e0d9ac21ac2e90c6f01204d977b8027eb0f50222c90e4734`
- Reason: applies after patch 26; backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). Backports of upstream
  Oniguruma: the parser tests for the end of the pattern before it reads after `\k`, `\g`,
  `\p{` (dc0a23e, #20), `(?<` (68c3955), `[\p{` (5186c7c), and `\k<a+`, `\k<a+1`, `\k<-`
  (562bf48, 162cf91). It read the terminator of the pattern there, and two to four bytes for
  a pattern in UTF-16LE. Each place keeps the warning, the token or the error that reading
  the terminator led to, where upstream changed them: `\k` and `\g` at the end warn and stay
  literal, `(?<` at the end is "group name is empty" and `\k<a+` at the end "invalid group
  name".

## 35. engine-matcher-bounds

- File: `regexec.c`
- Patch: `patches/engine-matcher-bounds.patch`
- Patched SHA-256: `037f751d2a10a3eb50e5a72654957517911ba59e375a3e50766087f2c8c05822`
- Reason: backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). Backports of upstream
  Oniguruma to the matcher, as patch 7 of the tree of Oniguruma 5.9.6 has them: `OP_EXACT1`
  checks the end of the subject before it reads (690313a, CVE-2017-9224);
  `forward_search_range` fails when `reg->dmin` leads past the end and does not step before
  the start by `reg->dmax` (9690d3a, CVE-2017-9227; b690371, CVE-2017-9229); the
  back-reference opcodes do not look for the previous character past the end after an empty
  match (34c8cdd, 1cb2c31); `OP_CCLASS_MB` checks the end before it reads (6d248fe);
  `OP_MEMORY_END_PUSH_REC` takes the stack index before the push that can move the stack
  (a8ed18b); `str_lower_case_match` stops at the end of the pattern's text (d3e4029,
  CVE-2019-19246); and, as the tree of Oniguruma 5.9.6 does without an upstream commit,
  `OP_PUSH_OR_JUMP_EXACT1` and `OP_PUSH_IF_PEEK_NEXT` check the end before they compare.
  None of these changes a result.

## 36. engine-distance-value

- File: `regcomp.c`
- Patch: `patches/engine-distance-value.patch`
- Patched SHA-256: `f9972fb4e00b250417e84ac5dfc304e5cfebb7041467ae0d3d3773f5a66e56e8`
- Reason: applies after patch 27; backport for the review of later security fixes (change
  `fuzz-and-review-ruby-1-9-2`, `docs/vendored-security.md`). Backport of upstream Oniguruma
  9574bc1: `distance_value` keeps the distance unsigned; as a negative number it indexed its
  table far before the start, and nested repeats such as
  `(?:(?:(?:a{0,100000}){0,100000}){0,100000})bc` ended the process when they were compiled,
  which `tests/backports.rs` shows.

## 37. engine-backward-search-dmin

- File: `regexec.c`
- Patch: `patches/engine-backward-search-dmin.patch`
- Patched SHA-256: `037f751d2a10a3eb50e5a72654957517911ba59e375a3e50766087f2c8c05822`
- Reason: applies after patch 35; backport of the backward half of upstream 33e5b47c ("Fix
  handling of reg->dmin in Regex matching", which distributions name for CVE-2024-27282;
  change `check-the-least-length-in-backward-searches`, the row of CVE-2024-27282 in
  `docs/vendored-security.md`). `backward_search_range` returns "not found" when
  `reg->dmin` exceeds the length of the subject, before it adds `reg->dmin` to the start of
  the range. The forward half is the check of CVE-2017-9227 in patch 35. A plain pattern
  longer than the subject never got there, because the threshold length turns it away
  first; a least distance that does not fit `OnigDistance` is the infinite one, for which
  no threshold is set, and the addition then wrapped around the address space, which the
  sanitized build reported. No result changes; `tests/backports.rs` shows it.

## 38. correctly-rounded-math

- File: `math.c`
- Patch: `patches/correctly-rounded-math.patch`
- Patched SHA-256: `b9716016a8e7f326fc35c119b0c7a8ee8b674540b7a8717b986a78c635f52c71`
- Reason: a save holds floats that `Math` computed, and the C library's results differ
  between versions of glibc and between operating systems (change
  `round-the-math-functions-correctly`, "Floats in saves" in `docs/decisions.md`). `math.c`
  includes `vkrgss_math.h` of `third_party/core-math/vkrgss`, whose macros make every call
  of `atan2 cos sin tan acos asin atan cosh sinh tanh acosh asinh atanh exp log log2 log10
  cbrt hypot erf erfc tgamma lgamma_r` call the correctly rounded function of the vendored
  CORE-MATH; for a special argument the C library's function is still called, and the
  checks of the domain before the calls are unchanged. `lgamma_r` takes the sign from
  `signgam`, which CORE-MATH's `lgamma` sets, while the call holds the interpreter's lock.
  The header defines `log2`, so the fallback under `#ifndef log2` stays out as before. No
  other line changes. `sqrt`, `frexp`, `ldexp` and `modf` are exact and stay with the C
  library. Patches 39 and 40 do the same for `numeric.c` and `bignum.c`.

## 39. correctly-rounded-pow-numeric

- File: `numeric.c`
- Patch: `patches/correctly-rounded-pow-numeric.patch`
- Patched SHA-256: `4848fb4ee9334a37fceef7c6f55cb1eaf8bc4b943d380d8efd7d1b89956c6947`
- Reason: as patch 38, for `pow` in `Float#**` and in `Fixnum#**` with a float exponent.

## 40. correctly-rounded-pow-bignum

- File: `bignum.c`
- Patch: `patches/correctly-rounded-pow-bignum.patch`
- Patched SHA-256: `827cacc9a5e4a346d8fd2d3b958db2399e0509fbfd0c1856f33fe47115a982f0`
- Reason: as patch 38, for `pow` in `Bignum#**` where the result is a float.

## 41. bignum-int-gc-guard

- File: `bignum.c`
- Patch: `patches/bignum-int-gc-guard.patch`
- Patched SHA-256: `827cacc9a5e4a346d8fd2d3b958db2399e0509fbfd0c1856f33fe47115a982f0`
- Reason: applies after patch 40; a wrong sum under `GC.stress` on aarch64 (change
  `guard-values-held-by-inner-pointers`; `test_utime` of
  `test/ruby/test_file_exhaustive.rb`, two tests of `test/ruby/test_time_tz.rb`, GCC 13 at
  `-O2`). `bigadd_int` and `bigsub_int` take `xds = BDIGITS(x)`, call `bignew`, and read
  `xds` afterwards. A Bignum of two digits keeps its digits in the object, so `xds` points
  into `x`; the compiler kept only that pointer across the call, `x` was collected and the
  new Bignum took its slot, and the sum read its zeros: `(2**63 - 1) + 1` gave 1, which made
  every time after 1970 "out of system range". `RB_GC_GUARD(x)` keeps `x` to the end of
  both functions. The study's guard in the branch for `SIZEOF_BDIGITS == SIZEOF_LONG` is
  left out: that branch is not compiled where `BDIGIT` is `unsigned int`, as on every
  target of vkrgss. Patch 42 is the other guard of the change.

## 42. marshal-userdef-gc-guard

- File: `marshal.c`
- Patch: `patches/marshal-userdef-gc-guard.patch`
- Patched SHA-256: `20c85c913e1f94be2ebad4b7616ebce9ccdc2ea3cd3ea7856efe1e188204dda6`
- Reason: "can't dump anonymous ID 0" under `GC.stress` on aarch64 (change
  `guard-values-held-by-inner-pointers`; `test_marshal_nsec` and `test_marshal_ivar` of
  `test/ruby/test_time.rb`, two tests of `test/ruby/test_marshal.rb`, GCC 13 at `-O2`). In
  the branch of `_dump` of `w_object`, after `has_ivars(v, ivtbl2)` the string `v` is used
  only through `RSTRING_PTR(v)` and `ivtbl2`; `w_class` allocated, `v` was collected and its
  generic instance variables freed, and `w_ivar` walked the freed table. `RB_GC_GUARD(v)`
  keeps the string until `w_ivar` has returned, as patch 23 of the tree of Ruby 1.8.7 does.
  Patch 41 is the other guard of the change.

## 43. bignum-inner-pointer-audit

- File: `bignum.c`
- Patch: `patches/bignum-inner-pointer-audit.patch`
- Patched SHA-256: `827cacc9a5e4a346d8fd2d3b958db2399e0509fbfd0c1856f33fe47115a982f0`
- Reason: applies after patch 41; the audit of the change
  `guard-values-held-by-inner-pointers` (its `design.md` lists every place it looked at).
  Where a function reads the digits of a Bignum through a pointer after a call that
  allocates, and does not use the Bignum itself afterwards, the Bignum may be collected as in
  patch 41. `RB_GC_GUARD` keeps it in `bigmul1_balance` (`y`, which `rb_big_pow`
  passes as a temporary), `bigdivrem` (`x` and `y`, which `big_fdiv` passes as
  temporaries and does not use again), and `bigand_int`, `bigor_int` and `bigxor_int`
  (`x`, a clone that the caller may drop in a tail call). No failure was seen; no result
  changes. Patches 44 to 46 are the other guards of the audit.

## 44. string-inner-pointer-audit

- File: `string.c`
- Patch: `patches/string-inner-pointer-audit.patch`
- Patched SHA-256: `c5a038e8e2de7785c3743a4b5a3de180222da50a2ad5faad46d054f2f162e48a`
- Reason: applies after patch 32; the audit of the change
  `guard-values-held-by-inner-pointers`, as patch 43 for strings. `RB_GC_GUARD` keeps
  a string that is read through `RSTRING_PTR` after a call that allocates: the
  replacement in `rb_str_sub_bang` across `RESIZE_CAPA`; the frozen copy of
  `rb_str_each_codepoint` across `rb_yield`; the result of `rb_inspect` in
  `rb_to_id`, whose bytes `rb_raise` reads after it has allocated (the guard is on the
  assignment, since a guard after a call that does not return would be dropped); and the
  results of `to_str` that `tr_trans` and `tr_setup_table` read across
  `rb_hash_new` and `rb_hash_aset`. `StringValue` takes the address of its variable,
  but within `string.c` the compiler may inline it, so that does not hold the string. No
  failure was seen; no result changes.

## 45. sprintf-signed-bignum-gc-guard

- File: `sprintf.c`
- Patch: `patches/sprintf-signed-bignum-gc-guard.patch`
- Patched SHA-256: `9a2e1424431af3d098056d263e177b707c048270ed5633bdc0801ca7c443020c`
- Reason: applies after patch 31; the audit of the change
  `guard-values-held-by-inner-pointers`, as patch 43 for `format`. A signed integer
  conversion of a Bignum (`%d`, `%+x`) reads the digits of `rb_big2str` through
  `s` after `CHECK` and `PUSH` have resized the result, which can collect; the
  string is now also stored in the volatile `tmp1`, as the unsigned branch next to it
  already does. No failure was seen; no result changes.

## 46. pack-ber-buffer-gc-guard

- File: `pack.c`
- Patch: `patches/pack-ber-buffer-gc-guard.patch`
- Patched SHA-256: `d3a5bba42951dc5676a0fcbf9aa1b1cb699e5760af5d0df746ac61dcf8663747`
- Reason: applies after patch 30; the audit of the change
  `guard-values-held-by-inner-pointers`, as patch 43 for `Array#pack`. The branch `w`
  (BER-compressed integer) builds its bytes in the local string `buf` and passes
  `RSTRING_PTR(buf)` to `rb_str_buf_cat`, which resizes the result, and can collect,
  before it copies them; `buf` is not used afterwards and nothing else holds it.
  `RB_GC_GUARD(buf)` keeps it until the copy is done. No failure was seen; no result
  changes.

## 47. null-operands

- File: `parse.y`
- Patch: `patches/null-operands.patch`
- Patched SHA-256: `dcd851d90cdde49d9a27c64ea688bfa76be187aa299b034337df17539f03753d`
- Reason: applies after patch 23; found by the fuzz target `parse192`, the first in the
  nightly run (change `make-the-nightly-checks-pass`). After a syntax error some rules give
  a null node as an expression: the statement that assigns to a constant with an operator
  and a command, as `A::B += p 1`, which parentheses pass on, and a word list whose
  content is no valid UTF-8, as `%W,` and the byte `0xce`. The rules of `..` and `...`
  looked at the type of such an operand, as in `(A::B += p 1)..1`, and so did the rule of
  `=~` after `match_op`, as in `%W,\xce=~/`; both read the null pointer. They test the
  pointer first, as `match_op` does and as patch 30 of the tree of 1.8.7 does for the
  ranges. Inputs: `crates/vkrgss-ruby192-sys/fuzz/fixed/parse/{range-null-operand,match-null-operand}.rb`.

## 48. null-operands-generated

- File: `parse.c`
- Patch: `patches/null-operands-generated.patch`
- Patched SHA-256: `57cd6e133392ae4f77a05d634e29d6abba2675c99d37cc57a1716ca54ede240b`
- Reason: follows patch 47. The same lines of the pre-generated parser, changed by hand.
