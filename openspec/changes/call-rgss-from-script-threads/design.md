# Design

## Context

- `crates/vkrgss-binding/src/guest.rs`: `GUEST`, `FREED`, `CLASSES` are `thread_local!`;
  `Guest::with` panics with "a guest is installed on the interpreter thread" on another thread;
  `defer_free` pushes into the calling thread's `FREED`. The same pattern is in `element.rs`
  (`FREED` of scene elements), `input.rs` (`NAMES`), `stats.rs`.
- `crates/vkrgss-ruby-sys/src/host.rs` and `crates/vkrgss-ruby192-sys/src/host.rs` (`run`, about
  line 188): `Ruby::new()` is justified by "Ruby only calls methods on the interpreter thread".
- The binding never releases the interpreter lock (no `rb_thread_blocking_region` or similar in
  `vkrgss-binding`), so while a host function runs no other Ruby thread runs. Ruby 1.8.7 has
  green threads: every Ruby thread is the interpreter's OS thread.
- `docs/decisions.md` (Ruby, "Script threads"): threads of RGSS3 run; a run with a live script
  thread is not claimed reproducible; a save state is refused while one is alive.
- `crates/vkrgss-binding/src/tilemap.rs:57-68,308`: `tilemap`, `autotile0..6` of
  `TilemapAutotiles`. RGSS2 names were measured with
  `ref/tools/oracle/measurements/vx-v3-save-lazy-probe.rb` (see `vx-v3-save-REPORT.md`).

## Decisions

- **Measure the original first.** A probe for a scratch VX Ace game runs, each in a fresh
  `Thread.new { ... }.value` with the exception caught: `Bitmap.new(8, 8)` and a `fill_rect`,
  `Sprite.new` with that bitmap, `Graphics.brightness`, `Graphics.update` (once),
  `Input.update` with `Input.press?(:C)`, `Audio.se_stop`, and `GC.start` after dropping a
  bitmap made in the thread. It writes each result or exception class. It runs on the original
  through `cargo xtask oracle` (Wine only through that, per `../../CLAUDE.md`), and on vkrgss.
  The table of results goes into `docs/compatibility.md` and under "As built" here. If every
  call works on the original (expected), the binding makes every RGSS class work from a script
  thread. If some call raises on the original, the binding raises the same class there, and the
  scenario "A bitmap and a sprite in a thread" of this change's spec is corrected to the
  measured result before it is implemented.
- **One state of the process.** `GUEST`, `CLASSES`, the two `FREED` queues and the other
  thread-locals of the binding that hold interpreter state become statics behind a cell type
  of the binding (for example `InterpreterCell<T>`) whose access requires a `&Ruby`, the token
  of a thread that holds the interpreter. Its `Sync` is justified by the interpreter lock: Ruby
  1.9.2 runs one Ruby thread at a time and the binding does not release the lock inside an
  access; Ruby 1.8.7 has one OS thread. The re-entrancy panic of `Guest::with` stays. A free
  function, which can run during a collection on any Ruby thread, pushes into the shared queue,
  which the next access on any thread drains. `stats.rs` counters, if they are per frame of the
  interpreter, follow the same rule.
- **Safety comments.** The comment at `Ruby::new()` in both `host.rs` says Ruby calls a host
  function only on a thread that holds the interpreter (the main thread of 1.8.7, or a thread
  holding the lock of 1.9.2). No behaviour change in the sys crates.
- **What a thread sees.** `Graphics.update` from a thread waits for the tick as from the main
  script; while it waits, no other Ruby thread runs, as before. Whether the original lets other
  threads run during `Graphics.update` is outside this change (Follow-up if the probe shows it).
- **Tilemap names of RGSS1.** A probe like `vx-v3-save-lazy-probe.rb`, written for a scratch XP
  game, notes the number of a new name before and after `Tilemap.new`, `t.autotiles[0]`,
  `t.autotiles[0] = nil`, and lists the names that appeared. It runs on the original XP runtime
  through `cargo xtask oracle` and on vkrgss. If the original interns the same names at the same
  moments, the test pins the numbers; otherwise the binding uses names of the start-up symbol
  table, or interns the original's names at its moments, as for RGSS2.
- **Tests.** `crates/vkrgss-binding-rgss3/tests/script_threads.rs` (new):
  `rgss_classes_work_from_a_script_thread` (each call of the probe in a thread, expected
  results as measured), `collection_on_a_script_thread_frees_bitmaps_and_elements` (a thread
  drops bitmaps and sprites and runs `GC.start`; after the next `Graphics.update` the arena and
  scene counts are back), and a test that the main script still works after a thread ended.
  `crates/vkrgss-binding/tests/run_game_lazy_names.rs` (or `run_game_symbols.rs`):
  `tilemap_names_of_rgss1_as_measured`.

## Out of scope

Running other Ruby threads while `Graphics.update` waits; making runs with script threads
reproducible; save states while a script thread is alive (still refused).

## Follow-ups

None yet.
