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

### Measurements (tasks 1.1 and 1.2): run on the originals on 2026-10-27

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
| 0 | the scenario: `Bitmap.new(8, 8)`, `fill_rect`, `Sprite.new`, `bitmap=`, `[b.width, Graphics.brightness]` | `[8, 255]` | `Exception: panic in a host function: a guest is installed on the interpreter thread` | `[8, 255]` |
| 1 | `Bitmap.new(8, 8)`, `fill_rect`, `get_pixel` | `[8, 8, 255.0, 0.0, 255.0, 0.0]` | the same panic | `[8, 8, 255.0, 0.0, 255.0, 0.0]` |
| 2 | `Sprite.new` with that bitmap, `x = 3` | `[8, 3, false]` | the same panic | `[8, 3, false]` |
| 3 | `Graphics.brightness` | `255` | the same panic | `255` |
| 4 | `Graphics.update` once (frames counted) | does not return: no frame is reached in 60 s (run last, see below) | the same panic | `1` |
| 5 | `Input.update`, `Input.press?(:C)` | `false` | `false` (the probe's `Input`) | `false` |
| 6 | `Audio.se_stop` | `nil` | the same panic | `nil` |
| 7 | `GC.start` after a dropped bitmap | `:done` | the same panic | `:done` |
| 8 | `Viewport`, `Plane`, `Window`, `Tilemap`, `Font`, `Table`, `Rect`, `Tone` | `[3, 2.0, true, 5, 32, false, false, false]` | the same panic | `[3, 2.0, true, 5, 32, false, false, false]` |
| 9 | a hundred sprites with bitmaps dropped, `GC.start` | `:done` | the same panic | `:done` |
| 10 | the main script afterwards: `Bitmap`, `Sprite`, `dispose` | `:done` | `:done` | `:done` |

Tilemap names of RGSS1 on vkrgss before this change (serial of the marker, `>> 3`, and the
new names):

| # | Step | Original | vkrgss |
|---|---|---|---|
| 0 | nothing | 2070 | 2070 |
| 1 | `Tilemap.new` | 2071 | 2072, `tilemap` (16569) |
| 2 | `t.autotiles[0]` | 2073, `at0` (16577) | 2074, `autotile0` (16585) |
| 3 | `t.autotiles[0] = nil` | 2074 | 2075 |
| 4 | `t.autotiles[1] = Bitmap.new(8, 8)` | 2078, `owner` (16601), `observer` (16609), `at1` (16617) | 2077, `autotile1` (16609) |
| 5 | a second `Tilemap.new` and its `autotiles[0]` | 2079 | 2078 |
| 6 | `Marshal.dump` of a tilemap | 2080, `TypeError` | 2079, `TypeError` |
| 7 | `t.dispose` | 2081 | 2080 |

So vkrgss interns `tilemap` at `Tilemap.new` and `autotileN` at the first read or write of
slot N; each takes a number of the sequence. Whether the original does the same is task 1.2.

### Runs on the originals (2026-10-27, from the owner's session)

The flow's sandbox could not run `bwrap`; the owner's session ran the probes through
`cargo xtask oracle --original-only` with the run-time package of VX Ace
(`--rtp RPGVXAce=../../ref/ace/rtp` and its two VL Gothic fonts; without them the original
stops at "Unable to find VL Gothic font") and, for XP, `--rtp Standard=../../ref/app`. Outputs,
outside the repository:

- `../../vkxp/target/call-rgss-threads/ace-301/original/game/threads.txt`: the probe as written
  stopped after step 3; `Graphics.update` called in a thread never returned.
- `../../vkxp/target/call-rgss-threads/ace-301-v2/original/game/threads.txt`: the same probe
  with the step `Graphics.update once` moved after the main script's step
  (`local/probes/threads-probe-v2.rb`, game `local/probes/game-ace-v2`). Every other call works
  in a thread and gives what vkrgss gives after this change; the `Graphics.update` step again
  never returned and the original reached no frame in 60 s.
- `../../vkxp/target/call-rgss-threads/xp-103/original/game/tilemap-names.txt`
  (`RGSS103J.dll`): the column "Original" of the second table.

So the original of RGSS3 runs every class from a script thread except `Graphics.update`,
which hangs there. vkrgss keeps returning after one frame: copying a hang serves no game, and
the requirement asks that a thread's calls work (recorded in `docs/compatibility.md` by task
4.1). The original of RGSS1 interns no name of its own at `Tilemap.new` (one number), `at0`
at the first read of slot 0, and `owner`, `observer` and `at1` when a bitmap is set in slot 1;
vkrgss interns `tilemap`, `autotile0` and `autotile1` instead, so task 3.1 follows the
decision "Tilemap names of RGSS1".

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

### Tilemap names of RGSS1 (task 3.1)

- The variables of a `TilemapAutotiles` object that hold its bitmaps are named `at0` to
  `at6`, the original's names (`RGSS104E.dll` formats them from `at%d` in `[]` and `[]=`),
  interned at the first read or write of their slot as there. The variable that holds the
  tilemap is `autotiles`, a name of the start-up symbol table, because `Tilemap.new` of the
  original interns none; the same rule as `bitmaps` of RGSS2.
- `tilemap_names_of_rgss1_as_measured` is in a test binary of its own,
  `crates/vkrgss-binding/tests/run_game_tilemap_names.rs` (with its file over Ruby 1.9.2 in
  `vkrgss-binding-rgss3`, listed in `ruby192.allow` with the other tests of Ruby 1.8's
  numbering), because the interpreter starts once per process and the numbers need a fresh
  one; the two files the design names each hold the one test of their process. Its probe is
  the steps of the scenario of `local/probes/tilemap-names-probe.rb`, with the same
  identifiers. The oracle runs its own probe as the first script, which interns names this
  test's game does not have (2070 there, 2039 here for the first step), so the test compares
  how far each number is from the first step's marker: `+1` after `Tilemap.new`, `at0` at `+2`
  and `+3` after `t.autotiles[0]`, `+4` after `t.autotiles[0] = nil`, as on the original.
  Before the change it gave `+2` with `tilemap` and `+4` with `autotile0` at `+3`.
- Step 4 of the probe (`t.autotiles[1] = Bitmap.new(8, 8)`) is outside the scenario. There the
  original also interns `owner` and `observer`; `[]=` of the autotiles does not refer to them
  in `RGSS104E.dll`, so they come with `Bitmap.new`, the first of the probe, as on the original
  of VX (`wrap::intern_owner_names`), which vkrgss does only in RGSS2. That is not a name of a
  tilemap; see "Follow-ups".

### Documents (task 4.1)

`docs/compatibility.md` has the table of the script-thread probe on the original 3.0.1.1 and
on vkrgss under "RPG Maker VX Ace", with `Graphics.update` in a thread as a known difference,
and the names of a tilemap of XP after the note on `Tilemap` of XP, with `owner` and
`observer` as a known difference. `docs/decisions.md` ("Script threads") says that
`Graphics.update` from a thread returns where the original hangs; `CHANGELOG.md` says both.
The tests that check the documents (`xtask`'s `doc_links`, `former_name`) pass.

### Checks of 2026-10-27 (task 5.1)

At `e172139b`: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exited 0, all steps
passed; `cargo xtask xarch check` against a baseline of `8f028ad5` (the proposal), with the
ten comparison games of the workspace's `xarch.toml`, printed `all identical` (84 frames, 17
saves); `openspec validate call-rgss-from-script-threads --strict` passed. Task 3.1 has
changed nothing yet, so these are to be repeated once it has.

Repeated at `c83a3f77`, after tasks 3.1 and 4.1: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo
xtask gate` exited 0, all steps passed; `cargo xtask xarch check` against the same baseline
and games printed `all identical` (84 frames, 17 saves), so the names of a tilemap of XP
moved no number that those saves hold; `openspec validate call-rgss-from-script-threads
--strict` passed. The sandbox of the flow could not write `/run/user/1000`, so the check ran
with `XDG_RUNTIME_DIR` set to the build's `TMPDIR`, where the guest socket goes; without it
the two games of VX Ace stopped at "cannot create the directory".

## Follow-ups

- **Measurements need the owner's shell.** `bwrap` cannot make its namespaces inside the
  sandbox of a flow; runs of `cargo xtask oracle` on the originals are done from the owner's
  session (see "Runs on the originals").
- **`owner` and `observer` in RGSS1.** The original XP runtime interns `owner` and `observer`
  at the first `Bitmap.new` of the tilemap probe (step 4, "Runs on the originals"); vkrgss
  interns them only in a game of RGSS2, at the calls measured on VX. Which calls of RGSS1
  intern them was not measured, and doing so would move the numbers of every later name of
  an XP game, so it belongs to a change of its own with a probe like that of RGSS2.
