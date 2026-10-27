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

## As built

### Measurements (tasks 1.1 and 1.2): not run on the originals

On 2026-10-27 neither original runtime could be run. `cargo xtask oracle` runs Wine only
inside `bwrap`, and the flow that implemented this change ran under a seccomp filter with
`no_new_privs` and no capabilities (`/proc/self/status`: `Seccomp: 2`, `NoNewPrivs: 1`,
`CapEff: 0`), so that `bwrap --ro-bind / / true` already fails with
`Failed to make / slave: Operation not permitted`; the oracle stopped at `wineboot -i` with
that message. Wine is never run outside that sandbox (`../../CLAUDE.md`). Tasks 1.1 and 1.2
stay open; see "Follow-ups".

The probes were written and run on vkrgss through `cargo xtask oracle --vkrgss-only`; they and
their outputs are under `local/` of the clone, outside the repository:

- `local/probes/threads-probe.rb`, the script of a scratch VX Ace game
  (`local/probes/game-ace`: `Game.exe` of Crysalis, `RGSS301.dll` 3.0.1.1 of In Search of
  Immortality, `RTP=` empty), each call in a fresh `Thread.new { ... }.value` with the
  exception caught, as decided above, with the calls of `Viewport`, `Plane`, `Window`,
  `Tilemap`, `Font`, `Table`, `Rect` and `Tone` added and a hundred sprites with bitmaps
  dropped in a thread, then one call in the main script after the threads. Output
  `threads.txt`: `local/oracle/ace-before/vkrgss/game/threads.txt` before this change,
  `local/oracle/ace-after/vkrgss/game/threads.txt` after task 2.1. Built-in scripts: a stand-in
  that defines `RPG` only.
- `local/probes/tilemap-names-probe.rb`, the script of a scratch XP game
  (`local/probes/game-xp`, `Library=RGSS103J.dll`, the DLL the start-up symbol table of RGSS1
  was measured on), as `vx-v3-save-lazy-probe.rb`: the serial of a marker name after each
  step and the names that appeared. Output `local/oracle/xp-before/vkrgss/game/tilemap-names.txt`.

What the original of VX Ace did with threads before this change, from the measurement of
A0 (`ace-v0-thread-probe.rb`, `ace-v0-thread-log.txt`, 2026-10-13, both DLLs, kept outside
the repository): a thread that calls `Graphics.frame_count`, `Audio.se_stop`, `Bitmap.new(8,
8)` and `dispose` runs to its end; `Audio.me_fade` and `Audio.me_stop` from a thread work.
Nothing raised there. This is the expected outcome of the decision above, and the scenario
"A bitmap and a sprite in a thread" was left as written.

Script-thread probe on vkrgss (`--vkrgss-only`; the oracle's probe answers `Input` from its
schedule on both sides, which presses nothing here):

| # | Call in a thread | Original (this probe) | vkrgss before | vkrgss after |
|---|---|---|---|---|
| 0 | the scenario: `Bitmap.new(8, 8)`, `fill_rect`, `Sprite.new`, `bitmap=`, `[b.width, Graphics.brightness]` | not run | `Exception: panic in a host function: a guest is installed on the interpreter thread` | `[8, 255]` |
| 1 | `Bitmap.new(8, 8)`, `fill_rect`, `get_pixel` | not run | the same panic | `[8, 8, 255.0, 0.0, 255.0, 0.0]` |
| 2 | `Sprite.new` with that bitmap, `x = 3` | not run | the same panic | `[8, 3, false]` |
| 3 | `Graphics.brightness` | not run | the same panic | `255` |
| 4 | `Graphics.update` once (frames counted) | not run | the same panic | `1` |
| 5 | `Input.update`, `Input.press?(:C)` | not run | `false` (the probe's `Input`) | `false` |
| 6 | `Audio.se_stop` | not run | the same panic | `nil` |
| 7 | `GC.start` after a dropped bitmap | not run | the same panic | `:done` |
| 8 | `Viewport`, `Plane`, `Window`, `Tilemap`, `Font`, `Table`, `Rect`, `Tone` | not run | the same panic | `[3, 2.0, true, 5, 32, false, false, false]` |
| 9 | a hundred sprites with bitmaps dropped, `GC.start` | not run | the same panic | `:done` |
| 10 | the main script afterwards: `Bitmap`, `Sprite`, `dispose` | not run | `:done` | `:done` |

Tilemap names of RGSS1 on vkrgss before this change (serial of the marker, `>> 3`, and the
new names):

| # | Step | Original | vkrgss |
|---|---|---|---|
| 0 | nothing | not run | 2070 |
| 1 | `Tilemap.new` | not run | 2072, `tilemap` (16569) |
| 2 | `t.autotiles[0]` | not run | 2074, `autotile0` (16585) |
| 3 | `t.autotiles[0] = nil` | not run | 2075 |
| 4 | `t.autotiles[1] = Bitmap.new(8, 8)` | not run | 2077, `autotile1` (16609) |
| 5 | a second `Tilemap.new` and its `autotiles[0]` | not run | 2078 |
| 6 | `Marshal.dump` of a tilemap | not run | 2079, `TypeError` |
| 7 | `t.dispose` | not run | 2080 |

So vkrgss interns `tilemap` at `Tilemap.new` and `autotileN` at the first read or write of
slot N; each takes a number of the sequence. Whether the original does the same is task 1.2.

### Guest state of the process (tasks 2.1 and 2.2)

- `InterpreterCell<T>` in `guest.rs`: a static whose `get` takes a `&Ruby` and returns a
  reference that lives no longer than it. Its `Sync` rests on the interpreter lock as decided
  above. A guest is not `Send`, because the tables a tilemap shares with its `Table` objects
  are `Rc`s, so the cell does not ask for `T: Send`; instead `InterpreterCell::new` is
  `unsafe`, and its condition is that nothing in the value is tied to a thread, which holds
  for `Rc` and `RefCell`, whose accesses the lock orders. `GUEST`, `CLASSES` and the button
  names of `input.rs` are such cells; `Guest::with`, `guest::classes` and the functions of
  `scene_testing` take the `&Ruby` of the call, which every host function has.
- The two queues of collected objects (`FREED` of `guest.rs` and of `element.rs`) are
  `Mutex` statics, not cells: a free function runs inside the collector, on whichever Ruby
  thread it runs, and has no `&Ruby`. The next access of the guest, on any thread, drains
  both. The frame statistics of `stats.rs` are one `Mutex` static too, because they are not
  reached with a `&Ruby` either (a `Span` is entered and dropped around bitmap work); its
  unit tests work on a `Measuring` of their own, since the static is shared by every test of
  the binary. The counters of live elements of `scene_testing.rs` are atomics.
- The guest lives in the static until the process ends; the thread-local was dropped when
  the interpreter thread ended. Nothing depends on that drop: `play` disconnects the link
  itself, and finalisers that run after it (`at_exit` of RGSS1) still find the guest.
- The unit tests of `guest.rs` made a guest of their own on the test thread; they now use
  the harness's guest, and one queues a free on another thread.
- Comments outside the files the proposal names, a few lines each: the `# Safety` of
  `Ruby::new` and the documentation of `Ruby` in `interp.rs` of both interpreter crates say a
  thread that holds the interpreter, which the corrected comment in `host.rs` relies on; the
  section "Guest state" of `lib.rs`, and the safety comments of `wrap.rs` (`alloc`) and
  `rb.rs` (`perform`), say the same of the binding. No behaviour of the sys crates changed.

### Checks of 2026-10-27 (task 5.1, left open with tasks 1.1, 1.2, 3.1 and 4.1)

At `e172139b`: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exited 0, all steps
passed; `cargo xtask xarch check` against a baseline of `8f028ad5` (the proposal), with the
ten comparison games of the workspace's `xarch.toml`, printed `all identical` (84 frames, 17
saves); `openspec validate call-rgss-from-script-threads --strict` passed. Task 3.1 has
changed nothing yet, so these are to be repeated once it has.

## Follow-ups

- **The measurements on the originals (tasks 1.1 and 1.2, and 3.1 that depends on 1.2).**
  They need a run of `cargo xtask oracle` where `bwrap` can make its namespaces, which the
  sandbox of the flow of 2026-10-27 did not allow. With the probes above, in the owner's
  shell:

  ```sh
  cargo xtask oracle local/probes/game-ace --frames 30 --original-only \
      --locale en_US.UTF-8 --out ../../vkxp/target/call-rgss-threads/ace-301
  cargo xtask oracle local/probes/game-xp --frames 30 --original-only \
      --dll ../../ref/sys/RGSS103J.dll --rtp Standard=../../ref/app \
      --out ../../vkxp/target/call-rgss-threads/xp-103
  ```

  The first writes `original/game/threads.txt` (with 3.0.0.1, replace the DLL of the game by
  `games/ace-crysalis/System/RGSS300.dll`), the second `original/game/tilemap-names.txt`. If
  a call of the first raises on the original, the binding raises the same class there and
  the expected value of `rgss_classes_work_from_a_script_thread` changes; if the second
  differs from the vkrgss column above, task 3.1 does what the decision "Tilemap names of
  RGSS1" says and pins the numbers in `tilemap_names_of_rgss1_as_measured`. Task 4.1 then
  adds the measured table and the names of a tilemap of RGSS1 to `docs/compatibility.md`.
