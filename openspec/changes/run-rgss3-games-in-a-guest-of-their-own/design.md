# Design

## Context

Decided on 2026-10-13 (`docs/decisions.md`, "VX Ace"): a game of RGSS3 runs in a guest
executable of its own, `vkxp-guest-rgss3`, beside `vkxp`, found beside the program's own
path and checked for its version when it attaches (decision 2); `--guest thread` with a
game of RGSS3 is refused and a headless run of RGSS3 uses a process (decision 3); the RGSS
classes have one source (decision 5); a save state is refused while a script thread is
alive (decision 8). Everything below the executable exists on `main`: the interpreter
(`vkxp-ruby192-sys`), the classes over it (`vkxp-binding-rgss3`), the guest role
(`vkxp-guest`), the glue (`vkxp-guest-main`), the archive `Game.rgss3a` and the three
blocks of the built-in scripts file. No script of RGSS3 runs in vkxp yet: `refuse_rgss3`
of the glue ends every run of RGSS3 once the files are opened.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp/src/{main,process,args}.rs`, `crates/vkxp/tests/`, `crates/vkxp-guest`,
  `crates/vkxp-guest-main`, `crates/vkxp-binding/src/{runtime,kernel,graphics}.rs` and its
  prelude files, the two interpreter crates, `xtask/src/{deps,dist,gate,stress,extract}.rs`.
- The design of `build-the-binding-over-ruby-1-9-2` with its Follow-ups, and those of
  `vendor-ruby-1-9-2`, `hand-the-guest-process-its-settings` and
  `move-the-guest-role-into-a-library`.
- The measurements of the original runtime of RGSS3, outside the repository:
  `../../ref/tools/oracle/measurements/ace-v0-REPORT.md` ("the V0 report"),
  `ace-v1-api-REPORT.md` ("the API report"), `ace-v1-sprite-REPORT.md` ("the sprite
  report"), `../../ref/RGSS301_api.md` ("the method list"), and the study
  `../../ref/tools/ruby192-feasibility/REPORT.md` ("the study").

What is there today:

| Place | Today |
|---|---|
| `process.rs`, `guest_executable(runtime)` | the program itself for every runtime |
| `main.rs`, `guest_form` | a process in a window, a thread headless; the runtime is not asked |
| `vkxp-guest-main`, `run_guest` and `main.rs`, `run` | call `refuse_rgss3` after `guest_inputs` |
| `vkxp-guest-main`, `run_guest` | names `vkxp_ruby_sys::restart_thread_timer`, the one item it uses that `vkxp-ruby192-sys` lacks |
| `vkxp-binding`, `runtime.rs` | a game of RGSS3 is prepared, loaded and reported as one of RGSS2: `Reset`, `Section001`, `p` and `print` as modal messages, `load_data` as plain `Marshal.load` |
| `vkxp-binding`, the classes | `Runtime::Rgss3` takes the branch of RGSS2 everywhere (`rules`, `frame_size`, `FontDefaults::rgss2()`, `Squeeze::Rgss2`, `create_window_rgss2`) |
| `xtask dist` | builds `-p vkxp`, packs one executable and the licence texts of Ruby 1.8.7 |
| `cargo xtask extract-rpg` | writes `module RPG...`, `def rgss_main`, `def rgss_stop` of an image of RGSS3 into the user's file |

Five facts that the decisions and the brief of this change did not have:

1. **A real game does not reach its title with the classes of RGSS2.** The standard script
   set of VX Ace (read in the main test game, which uses it unchanged) calls, between the
   end of script loading and its first frame, in this order: `load_data` 14 times;
   `Tone.new` and `Color.new` without arguments (`Game_Screen`, reached from
   `DataManager.create_game_objects`); `Viewport.new` without arguments (`Scene_Base`);
   `Window#initialize(x, y, width, height)`, `padding=`, `padding_bottom=`, `tone`,
   `Tone#set(tone)`, `Rect.new` without arguments, `Rect#set(rect)`, `Color#set(color)`,
   `open?` (`Window_Base`, `Window_Selectable`); `Audio.bgm_play` with four arguments (the
   built-in `RPG::BGM#play`). Seven frames later it calls `Input.repeat?(:DOWN)`. Each of
   these raises with the classes as they are. They are the class surface of RGSS3, each
   with a measurement report of its own, and each can be built and tested in
   `vkxp-binding-rgss3` without a guest executable. So this change does not bind them, and
   the main test game is expected to load its 125 scripts and its database and to end at
   the first of them with an error report that names the script. The table under
   "Follow-ups: the changes that follow" orders the changes so that the first four reach
   the title.
2. **The capability `binding-over-two-interpreters` is not in `openspec/specs` yet**, and
   the main specifications of `workspace-structure` do not have the edits of
   `build-the-binding-over-ruby-1-9-2` yet (that change is complete and not archived). The
   deltas of this change repeat the requirements of `workspace-structure` with those edits
   and add nothing to `binding-over-two-interpreters`.
3. **Cargo gives a test the path of a binary of its own package only**
   (`CARGO_BIN_EXE_<name>`), artifact dependencies are not stable, and `cargo run -p vkxp`
   builds `vkxp` alone. But cargo puts the binaries of every package of a workspace into
   one directory, and builds the binary of each selected package that has an integration
   test before any test runs.
4. **The stress step `guest`** of `cargo xtask stress` runs `cargo test -p vkxp --test
   end_to_end` alone. A test of that file that needs the other executable would fail
   there.
5. **The script archive the editor of VX Ace writes labels its strings**: in the archive of
   the main test game the name and the deflated source of every entry carry `:E` true
   (seen on 2026-10-13), where the binding's test support writes them through the
   interpreter of the test.

## Goals / Non-Goals

**Goals:**

- `vkxp <game of VX Ace>` starts `vkxp-guest-rgss3`, and the scripts of the game run in it
  over Ruby 1.9.2 with the start-up behaviour measured on the original: the built-in
  scripts, `$RGSS_SCRIPTS`, each script evaluated as `{NNNN}` and compiled as UTF-8,
  `RGSSReset` and the two `__reset__`, the endings, the error report, `msgbox`, `p` and
  `print`, `load_data` and `save_data`.
- A generated game of RGSS3 runs headless from start to shutdown through both executables,
  with dumps, states and the sandbox, and is tested in the repository without a file of an
  original runtime.
- The release archive holds both executables.
- The documents say that VX Ace is experimental, and a run of RGSS3 says so once.
- XP and VX do not change: every existing test passes with its expected values unedited.

**Non-Goals:**

- The class surface of RGSS3: `Window`, `Tilemap`, `Font` and text, `Sprite`, `Viewport`,
  `Plane`, `Input`, `Audio`, the forms of `Color`, `Tone` and `Rect`. A game of RGSS3 sees
  the classes of RGSS2 in this change, and a frame it draws is expected to be wrong in the
  pixels where RGSS3 differs (text, window, tilemap, a turned sprite in a viewport).
- That a real game reaches its title screen (fact 1). The milestone A1 stays "Not started".
- Comparing with the original: `cargo xtask oracle` for RGSS3, pixels, saves.
- `Win32API` and `DL` over Ruby 1.9.2, the position of `Audio.bgm_play`, encodings beyond
  what `vkxp-ruby192-sys` has, the start-up symbol table.
- An option or variable that names the guest executable; a guest on a thread for RGSS3.
- Any change of the behaviour of Ruby 1.8.7, of the protocol or of the host crates, and of
  `third_party/` unless a fault of a copy of the process needs a recorded patch.

## Decisions

1. **The second package of the glue.** `crates/vkxp-guest-main-rgss3`, package
   `vkxp-guest-main-rgss3`, library `vkxp_guest_main_rgss3`, defined exactly as
   `vkxp-binding-rgss3` was (decision 1 of `build-the-binding-over-ruby-1-9-2`):

   ```toml
   [package]
   name = "vkxp-guest-main-rgss3"
   description = "The main of a guest of vkxp over Ruby 1.9.2: the source of vkxp-guest-main"
   build = "build.rs"

   [lib]
   path = "../vkxp-guest-main/src/lib.rs"
   doc = false
   doctest = false

   [dependencies]
   # As vkxp-guest-main, with these two lines different:
   vkxp-binding = { package = "vkxp-binding-rgss3", path = "../vkxp-binding-rgss3" }
   vkxp-ruby-sys = { package = "vkxp-ruby192-sys", path = "../vkxp-ruby192-sys" }
   ```

   The keys stay `vkxp-binding` and `vkxp-ruby-sys`, so the source goes on saying
   `vkxp_binding::` and `vkxp_ruby_sys::`. The directory has no `src`.
   - `build.rs` is that of `vkxp-binding-rgss3` (the `cfg` `vkxp_interpreter_absent` for
     every target but `x86_64-unknown-linux-gnu`), and `crates/vkxp-guest-main/src/lib.rs`
     gets the one line `#![cfg(not(vkxp_interpreter_absent))]` after its documentation.
   - The unit tests of `lib.rs` run in both packages. `vkxp-guest-main` has no `tests`
     directory today.
   - The two tests of `xtask` that keep `vkxp-binding` and `vkxp-binding-rgss3` in step
     take a list of pairs and get this pair as the second: the dependencies of the two
     manifests are equal after replacing the interpreter crate, the binding package and
     the package's own name; every `.rs` file of `crates/vkxp-guest-main/tests`, should
     there be one, has its counterpart of the four lines.
   - `vkxp-guest-main-rgss3` joins `GUEST_CRATES` of `xtask/src/deps.rs`. Rule 5 holds by
     construction (its dependencies are those of `vkxp-guest-main` with the two packages
     replaced) and is checked over the real graph.

2. **What the glue needs of the interpreter crates.** Two free functions with one name and
   one signature in both crates, with the contract `restart_thread_timer` has today (to be
   called on the interpreter thread):

   | Item | `vkxp-ruby-sys` | `vkxp-ruby192-sys` |
   |---|---|---|
   | `restart_thread_timer()` | exists: arms the timer of the green threads in a copy | new: `rb_thread_reset_timer_thread()`, which forgets a timer thread the copy does not have; the patch `lazy-timer-thread` then starts one at the next `Thread.new`. Nothing happens when no script ever made a thread |
   | `threads_lost_in_a_copy() -> bool` | new: `false`; its threads are memory of the interpreter and are copied | new: `!rb_thread_alone()`; a Ruby thread of 1.9 is a thread of the system and does not exist in a copy (the study, 5b, rule 4) |

   The table "Differences from `vkxp-ruby-sys`" of `vkxp-ruby192-sys` and the paragraph on
   the shared surface of `vkxp-ruby-sys` are brought up to date. What the study names
   beside this (stopping the timer thread before `fork`, because it may hold its lock at
   that moment: inferred there and never hit) is a follow-up.

3. **Each executable runs the runtimes of its interpreter.** The glue gets
   `interpreter_runs(runtime) -> bool`: RGSS1 and RGSS2 over `RubyApi::Ruby18`, RGSS3 over
   `RubyApi::Ruby19`. `guest_on_thread` and `run_guest` end with an error that names the
   runtime and the interpreter when it is false, after `guest_inputs` and before a game
   file is opened or an interpreter started. `refuse_rgss3` and its tests go. The binding
   itself refuses one thing, a game of RGSS3 over Ruby 1.8.7 (`RunError::Interpreter` from
   `run_game`), so that the preparation of RGSS3, which is Ruby 1.9, is never evaluated
   there; games of RGSS1 and RGSS2 over Ruby 1.9.2 are what its shared tests run.

4. **The executable.** `crates/vkxp-guest-rgss3`, a package with one binary of that name,
   its build script (the same `cfg`) and no library. Dependencies: `vkxp-guest-main-rgss3`
   and nothing else. Its `main` holds one statement:

   ```rust
   fn main() -> ExitCode {
       vkxp_guest_main_rgss3::guest_process(env!("CARGO_PKG_VERSION")).unwrap_or_else(not_a_guest)
   }
   ```

   `not_a_guest` writes one line to the error stream (this is the guest executable of vkxp
   for games of RPG Maker VX Ace, and `vkxp <game directory>` starts it) and gives status
   2. It parses no option: `--help` and `--version` are what any other argument is. On a
   target without the second interpreter `main` writes that the executable is not built
   for the target and gives status 1.
   - It joins `GUEST_CRATES`: rule 1 then proves that it reaches no host crate, and a unit
     test of the rules gives them a graph in which it reaches `vkxp-host`. Rule 5 proves
     that it does not reach Ruby 1.8.7.
   - Its tests, `tests/guest_record.rs`, are those of `crates/vkxp/tests/guest_record.rs`
     for this executable (no such file, another version, not a record), and a record of a
     game of RGSS1, which decision 3 refuses. They need no Vulkan driver.
   - `unsafe` is not needed in it; `AGENTS.md` does not change in that respect.

5. **The program finds it beside itself.** `guest_executable(runtime)` of `process.rs`:
   for RGSS3 the file `vkxp-guest-rgss3` in the directory of `std::env::current_exe()`,
   links resolved as the system resolves them; when no file is there, an error that names
   the file and that directory and says that it is built with the program and lies beside
   it in the release archive. The error comes before the socket directory is made.
   - *Development build.* `target/<profile>/` holds both once both are built; `cargo run
     -p vkxp` builds one, and the error then names `cargo build -p vkxp-guest-rgss3`.
     `docs/running.md` and `docs/development.md` say so.
   - *Release archive.* Decision 16.
   - *No override.* No environment variable, option or configuration key names another
     place. Decision 2 of `plan-vx-ace` says "found beside its own path"; a packager who
     wants `libexec` gets a follow-up. A stand-in for tests needs none (below).
   - *Version.* The guest reads the record first and refuses one of another version, then
     states its own when it attaches, which the program checks. Both checks exist and both
     executables take the version of the workspace. A test of `xtask` holds that the two
     packages have one version. A whole executable of another version cannot be built
     inside one workspace; the two halves are tested each by itself: the guest executable
     given a record of version `0.0.0`, and the program given a connection that states
     `0.0.0` (which exists).

6. **How tests find the second executable.** They do not look for it: the program under
   test does, beside `CARGO_BIN_EXE_vkxp`, where cargo puts every binary of the workspace.
   - `crates/vkxp-guest-rgss3` has an integration test, so `cargo test --workspace` (the
     gate, CI) and `cargo test -p vkxp -p vkxp-guest-rgss3` build its binary before any
     test runs, from the sources as they are.
   - The tests of `crates/vkxp` that run a game of RGSS3 are a file of their own,
     `tests/rgss3.rs` (fact 4), empty on every target but `x86_64-unknown-linux-gnu`. Its
     first helper fails a test, before a child is started, when no file
     `vkxp-guest-rgss3` is beside the program, with a message that names `cargo test -p
     vkxp -p vkxp-guest-rgss3`.
   - Rejected: an artifact dependency (unstable, and the toolchain is pinned to stable); a
     build script of `vkxp` that builds the other package (a cargo inside a cargo that
     holds the build lock); a helper that runs `cargo build` from the test (it has to guess
     the profile and the target of the outer run, and builds the guest a second time with
     other features than the outer run chose, minutes in every gate); an environment
     variable for the path (an override the program would have to offer, see decision 5).
     What is given up: `cargo test -p vkxp` alone uses whatever guest executable an earlier
     build left. `docs/development.md` names the two commands that are always right.
   - *A missing executable and a stand-in* are tested with a copy of the program: the test
     copies the file of `CARGO_BIN_EXE_vkxp` into a directory of its own and runs the copy,
     alone (the error of decision 5) or with a two-line shell script named
     `vkxp-guest-rgss3` beside it that writes a line and exits with status 3 (the program
     reports how the guest process ended).

7. **The form of the guest.** `guest_form(chosen, headless, runtime)`: for RGSS3 a process
   whatever the kind of run when nothing was chosen. A choice of `thread` (option,
   `VKXP_GUEST` or the key `guest`) with a game of RGSS3 ends the run with status 1 and one
   message: a game of RPG Maker VX Ace runs with the guest in a process only, and
   `--guest process` or no choice is what to give. It is said once the runtime is known and
   before anything starts. `args::check` is not changed: `--states` still asks for
   `--guest process` by name in a headless run, for every runtime.

8. **Experimental.** `supported-runtimes` defines: supported when the milestones are done,
   experimental while the games start but the milestones are open, planned before. After
   this change the games of VX Ace start: their scripts run as far as the code goes. So VX
   Ace is experimental, in the first paragraph and the table of `README.md`, the first
   sentence of `AGENTS.md`, the openings of `docs/compatibility.md` and `docs/running.md`
   and the usage text, and the rows A1 to A3 stay "Not started". A run of RGSS3 says once,
   as a warning of the program (not of the guest), that support for RPG Maker VX Ace is
   experimental: the rule "Before VX is done" of `docs/decisions.md` did this for VX until
   V3, the requirement was removed at 0.2.0 only because VX became supported, and a player
   who starts a real game today meets an error after a few seconds (fact 1) and should
   have read why. The requirement comes back for RGSS3 and goes when A3 is reached. Older
   scenarios of `supported-runtimes` that say "VX Ace is planned" describe the documents
   at the time of their changes, as those on experimental VX do.

9. **The preparation of a game of RGSS3.** Chosen by the runtime of the run, not by the
   interpreter: the shared tests of the binding run games of RGSS1 and RGSS2 over Ruby
   1.9.2 and keep doing so.
   - `prelude.rb` keeps the definitions of RGSS1 and RGSS2 (`Reset`, `save_data`, `print`,
     `p`, `vkxp_load_scripts`) and is evaluated for those runtimes only. What every
     runtime gets (`Win32API`, `require` and `load`) moves to `prelude_shared.rb`,
     evaluated after it; the lines it leaves in `prelude.rb` stay as comment lines, as the
     two lines of Ruby 1.8 did, so that no line number of that file changes.
   - `prelude_rgss3.rb`, written for vkxp, is evaluated for RGSS3 in the place of
     `prelude.rb`. It defines `RGSSReset < Exception`, `save_data` (the dump written to
     the file opened `"wb"`, value `nil`), `load_data` over the host function of today
     (decision 11), `msgbox` and `msgbox_p`, and the loader `vkxp_load_scripts`
     (decision 10). It does not define `Reset`, `Hangup`, `print` or `p`.
   - `rgss_main` and `rgss_stop` are not vkxp's: they are two of the three blocks of the
     built-in scripts file, which is evaluated whole, under `(builtin)`, before the script
     archive is read, as for the other runtimes. A file without them is the user's matter:
     the game then ends with a `NoMethodError` in its last script.
   - `$TEST`, `$BTEST`, `$DEBUG` are set as for RGSS2, which is what the original does
     (API report 1.7: `test` sets `$TEST`, `btest` both, `$DEBUG` stays false). `$KCODE`
     is `nil` and `$:` is `["."]` by the interpreter crate.
   - `RGSS_VERSION` is `"3.0.1"` (ASCII-8BIT; V0 report 8, API report 3.3): the version
     vkxp follows by the decision on the DLL version. That it follows the DLL a game ships
     (`"3.0.0"` for a 3.0.0.x) comes with the glyph rule, which reads the same version
     record.
   - `p`, `print`, `puts` are Ruby's own and write to `$stdout`, which is the standard
     output of the guest process and so of the program (API report 1.7: no box, `p x`
     returns `x`). `$stdout` is unbuffered by the interpreter crate, so a copy that is a
     state repeats nothing. `--log-p` has nothing to change in a run of RGSS3 and is
     accepted.
   - The seed is set once, before the built-in scripts. A reset does not set it again:
     the loop of `rgss_main` is Ruby that vkxp does not see, and the original does not
     seed either. The requirement "Random seed" stays that of RGSS1 and RGSS2.
   - The functions that start processes: beside those `prelude_process_off.rb` turns off,
     a file `prelude_process_off_ruby19.rb`, evaluated over Ruby 1.9 when they are off,
     turns off `spawn`, `Process.spawn`, `Process.exec` and `Kernel#open` with a command
     (a String that starts with `|`); the list forms of `system`, `exec` and `IO.popen`
     are covered by the definitions that exist (API report 5.7 lists what works on the
     original). The sandbox, which is on in every run of RGSS3 unless turned off, refuses
     to execute a file whatever the script calls.

10. **Script loading as measured** (API report 1.1). `vkxp_load_scripts` of RGSS3:
    `$RGSS_SCRIPTS = load_data(path)`, then for each entry `s[3, 0] =
    Zlib::Inflate.inflate(s[2])`, value: the size, read once. `run_scripts` evaluates
    entry `i` under the name `{%04d}` of `i` (ASCII-8BIT, as every evaluation name
    already is) and reads `$RGSS_SCRIPTS[i][3]` when its turn comes, as today.
    - *Compiled as UTF-8.* The stored source is ASCII-8BIT, which `Zlib` returns. What is
      evaluated is a new string of the same bytes made with `rb::text_new`; the stored one
      is not touched. How the original does it is not known (API report 8, point 1); the
      effect is the measured one.
    - *The binding of main.* `evaluate` stays as it is (`eval` on the main object without
      a binding): `self` is main, constants, methods, globals and instance variables of
      main pass from script to script and locals do not, which is what was measured.
    - *A failed load.* The original shows `Failed to load script.` and ends with code 0;
      vkxp keeps `RunError::Archive` with the path and the reason, and a failure status.
      An empty archive runs nothing and ends with success, as on the original.

11. **`load_data` and `save_data`** (API report 2.1, 2.3). `load_data(name)` of RGSS3 is
    `Marshal.load(<the bytes of the file>, <proc>)`, called through Ruby, where the proc
    labels every String it is handed whose encoding is ASCII-8BIT as UTF-8 and returns
    its argument. The proc is made once in `prelude_rgss3.rb`; it compares
    `encoding.name` with `"ASCII-8BIT"`, because the encoding constants do not exist at
    start-up (API report 3.2). The bytes come from the game's files, archive included, as
    today. Left out: a frozen string (2 of 71 samples), the error texts of a loose file
    (`EOFError` where vkxp says `marshal data too short`), the lookup rules of 2.1, 2.2
    and 0.2.

12. **Reset** (API report 1.3 to 1.5).
    - A reset request of the host makes the next `Graphics.update` raise `RGSSReset` with
      the message `""` (`raise_pending` asks the runtime which class and which message).
    - `Graphics.__reset__` (arity 0, value `nil`): disposes every `Bitmap`, `Sprite`,
      `Viewport`, `Window`, `Plane` and `Tilemap` through the `dispose` of its RGSS class
      (the Ruby of `Graphics._reset` of today), sets the brightness to 255 and the frame
      rate to 60 and ends a freeze. `frame_count`, the size of the screen and
      `Graphics.visible` stay.
    - `Audio.__reset__` (arity 0, value `nil`): what `audio::stop_all` does.
    - `RGSSReset` that leaves a script (no `rgss_main` around it): `Graphics.__reset__` is
      called through Ruby, the audio is stopped, the input state starts afresh, and the
      scripts run again from the first preload script, the exception being the current
      error, as for the other runtimes. That the audio stops there is vkxp's rule for a
      reset; the report states only that `Graphics.__reset__` ran.
    - Left out: the further raise that arrives without a call of `Graphics`, `wait`,
      `fadeout`, `fadein` and `transition` cut short, `__reset__` waiting for the release
      of F12.

13. **Endings and the report** (API report 1.2, 1.6).
    - `exit` in any form, `raise SystemExit`, `abort`, and the end of the last script end
      the game with success: status 0, as measured.
    - An uncaught exception ends it with status 1. The original's code (the line number
      in 69 of 98 cases, "the meaning is not established") is not followed.
    - The report is built from the class, the message and the backtrace by the measured
      rules, in Rust, in `report` of `runtime.rs` for RGSS3:

      | Condition | Text |
      |---|---|
      | the class is `Errno::ENOENT` | `Unable to find file:\n\n<message without "No such file or directory - ">` |
      | the line is not 0 | `Script '<name>' line <line>: <class> occurred.\n\n<message>` |
      | the line is 0 | `<class> occurred.\n\n<message>` |
      | the message is empty | the same without `\n\n<message>` |

      Name and line are those of the first backtrace entry that does not start with
      `(eval)` (the last entry if all do); a name `{NNNN}` is replaced by
      `$RGSS_SCRIPTS[NNNN][1]`; for a `SyntaxError` whose entry has no line they are taken
      from the message, and the message is cut as the report's listing of 1.2 does. The
      text is written to the error stream and shown as a modal message, as for the other
      runtimes. Not followed: the empty box for a text of more than 1023 UTF-16 units.
    - Left out: `at_exit` handlers, which the original never runs; `exit!`.

14. **`msgbox` and `msgbox_p`** (API report 1.7). `msgbox(*args)`: each argument by
    `to_s`, `nil` as `nil`, joined with `$,`, `$\` appended, cut at the first NUL byte;
    `msgbox_p(*args)`: each by `inspect`, joined with line feeds. Both go through
    `vkxp_modal` (error stream, modal message, the panel until the host dismisses it) and
    return `nil`.

15. **States and the sandbox.**
    - A state of a guest of RGSS3 is a copy made by `fork` as any other; the requirement
      "A copy of the process between two frames" of `embedded-ruby-1-9-2` is what makes it
      work. `ProcessCopier::park` of the glue asks `threads_lost_in_a_copy()` first: when
      it is true it writes one line (the state of slot N is not saved: a script thread is
      alive, and a copy of the process would not have it) and returns `CopyFailed`, which
      the binding answers with the failure message the host shows. Loading is not
      refused: the copy that is loaded was made while there was none.
    - The sandbox needs nothing new. It is Landlock on the interpreter thread before the
      first script (files and TCP), with no filter of system calls (the study, 5c). The
      interpreter opens `/dev/urandom` for reading only. A thread a script makes, and the
      timer thread that starts with it, are made by the confined thread and inherit the
      restriction.
    - SIGVTALRM, which Ruby 1.9.2 takes once a script made a thread, is sent to a thread
      by name; the main thread of the guest executable blocks it before the interpreter
      starts (step 2 of the order of a guest process). Nothing to do.

16. **The release archive.** `cargo xtask dist` builds `-p vkxp -p vkxp-guest-rgss3` in
    one call, with `VKXP_STATIC_CRYPT=1`, which the build script of `vkxp-ruby192-sys`
    reads as that of `vkxp-ruby-sys` does; it reads the needed libraries of both
    executables and refuses a shared libcrypt in either. The archive gains
    `vkxp-guest-rgss3` beside `vkxp` and `licenses/ruby-1.9.2/COPYING`, `GPL` and `LEGAL`
    (the texts `THIRD-PARTY.md` names for that source).

17. **Tests.**
    - *In the binding*, files of `crates/vkxp-binding-rgss3/tests/` only, with a scripted
      host: the preparation, the loader, the reset by the host, the report, the modal
      messages, `load_data`. `Game::of(Runtime::Rgss3, ..)` of the test support writes
      the stand-in below as the built-in scripts file. `tests/interpreter.rs` of
      `vkxp-binding` (shared) holds decision 3 for both interpreters.
    - *Through both executables*, `crates/vkxp/tests/rgss3.rs`. A game is made by test
      code of that directory without an interpreter: `Game.ini`, the stand-in, and
      `Data/Scripts.rvdata2` written as Marshal 4.8, an Array of `[Fixnum, String,
      String]` whose two strings carry `:E` true (fact 5) and whose third is the zlib
      stream of the UTF-8 source (`flate2`, already in the lock file, as a development
      dependency). One test writes the two strings plain. The helpers both test files of
      `crates/vkxp` use move to `tests/support/`; no test of `end_to_end.rs` changes but
      the eight that hold the refusal, which move to `rgss3.rs` and expect a run.
    - *The stand-in built-in scripts of RGSS3*, the tests' own text, written for vkxp and
      not Enterbrain's:

      ```ruby
      # Stand-in for the built-in scripts of RGSS3, written for the tests of vkxp.
      module RPG
      end

      def rgss_main
        begin
          result = yield
        rescue RGSSReset
          Audio.__reset__
          Graphics.__reset__
          retry
        end
        result
      end

      def rgss_stop
        Graphics.update while true
      end
      ```

      No test asserts what `rgss_main` returns: that is a property of the user's file.
    - *Collector stress.* `cargo xtask stress` gets the step `guest-1.9.2`: `cargo test
      -p vkxp -p vkxp-guest-rgss3 --locked --test rgss3 --test guest_record` with
      `VKXP_GC_STRESS=1`, which the guest process inherits.

## Implementation notes

- A flow reads `../../ref/` and `../../games/` and copies nothing from them. No Wine and
  no original runtime is run. Scratch output goes under `target/` or `local/`.
- Before anything changes, keep under `local/` the lists of `cargo test --locked --
  --list` for `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest-main`, `vkxp-guest`,
  `vkxp-ruby-sys`, `vkxp-ruby192-sys`, and of `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test
  -p vkxp --locked -- --list`. At the end every name is there again, apart from the tests
  of `refuse_rgss3` and the eight tests of the refusal in `end_to_end.rs`, which have
  successors in `rgss3.rs`.
- Every command that reaches Vulkan has `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- A scenario that asks for a line of the diagnostics is met by the property (one line
  that names the thing), not by a fixed wording.
- `Cargo.lock` gains two packages; the gate builds with `--locked`.
- `vkxp` goes on depending on `vkxp-guest-main` and `vkxp-binding` (for `guest_inputs`,
  `guest_on_thread`, `frame_size`), never on a package over Ruby 1.9.2.
- The check of the local game, outside the tests and recorded under Follow-ups: copy
  `../../games/ace-crysalis` to `target/ace-check/`, build both executables, and run the
  copy headless with `--device llvmpipe --rtp RPGVXAce=../../ref/ace/rtp
  --builtin-scripts ../../ref/ace/rpg-rgss3.rb --frames 120 --dump 100=target/ace-check/
  frame.png`. Expected by fact 1: the report `Script 'Game_Screen' line ...:
  ArgumentError occurred.` and status 1. Whatever happens is written down, with the
  number of scripts that ran; nothing is fixed in the classes to get further.
- Documents: `README.md` (first paragraph, the row of the table, the table of crates and
  the sentence that no executable links the second interpreter; the file may not exceed
  200 lines, so lines are replaced, not added), `AGENTS.md` (first sentence),
  `docs/compatibility.md` (opening, and a short section "RPG Maker VX Ace" with what runs,
  what is RGSS2's still and the known differences of decisions 9 to 14),
  `docs/running.md` (the opening of "The runtime of a game", the paragraph on VX Ace, "The
  built-in scripts of RPG Maker VX Ace", "Guest process", "Sandbox", "Save states", the
  option table), `docs/architecture.md` (the two crates, the paragraph on the binding),
  `docs/decisions.md` (the table of crates, and under "VX Ace" the decisions 5 to 16 of
  this design in the document's own words), `docs/development.md` (decision 6, the stress
  step), `docs/releasing.md` (the two executables), `THIRD-PARTY.md` if it names what the
  archive holds, `CHANGELOG.md` under "Unreleased".
- The usage text: the line of `--runtime` says of `rgss3` that it is experimental, and
  the line of `--guest` that a game of RGSS3 runs in a process.

## Risks

- The preparation of RGSS3 meets a 1.8 idiom in a path no shared test took (the study,
  work package C). It shows in the tests of this change or in the check of the local game
  and is fixed in the shared source by a check of the runtime or of `rb::RUBY_19`.
- `cargo test -p vkxp` alone with a guest executable of an earlier build (decision 6).
- A copy made after a script thread has ended finds a timer thread that is not there.
  Decision 2 forgets it; the scenario "A thread after a copy" holds it.

## Follow-ups

### Left out of this change, with the measured source

Sections are those of the API report unless another report is named.

- Endings: `at_exit` handlers never run on the original, `exit!(n)` gives `n`: 1.6. The
  empty box above 1023 units and the exit code after an error: 1.2. `Failed to load
  script.` and code 0: 1.1.
- Reset: the raise without a call of `Graphics`, the waits and fades cut short,
  `__reset__` waiting for the release of F12, F2 and the title: 1.4.
- `load_data`: a frozen string, the errors of a loose file, a name without extension in
  the archive, the archive before a loose file, names that are not ASCII on the two DLL
  versions: 2.1, 2.2, 0.2.
- The last backtrace entry ``ruby:in `eval'``, `$0` and `$PROGRAM_NAME` false, the title
  `Untitled` without `Title=`: 1.1. Magic comments other than UTF-8: 1.1, 3.3.
- `RGSS_VERSION` and the glyph rule by the DLL a game ships: V0 report 8, the decision on
  the DLL version in `docs/decisions.md`.
- `require` and `load` as on the original, `require_relative`, `autoload`: 0, row
  "kernel"; V0 report 8.
- Processes: `IO.read` and its family with a command, `Process.daemon`: 5.7.
- Script threads: stopping the timer thread before a copy (the study, 5b, rule 2); how
  often a thread runs between frames (V0 report 8).
- An option, variable or key that names the guest executable, for packagers.
- A guest on a thread for RGSS3 (the study, option B).

### Found while implementing

- *The filesystem encoding* (outside the edits this design names, a few lines the scenario
  "Encodings, measured on the original" plainly needs). `__FILE__` takes the filesystem
  encoding of Ruby 1.9.2, which follows the default external encoding on Linux: UTF-8.
  The original finds no encoding for the code page of Windows and takes ASCII-8BIT (API
  report 3.1, `Encoding.aliases`; V0 report 8). `vkxp-ruby192-sys` now points the alias
  `filesystem` at ASCII-8BIT at start-up, as one more item of the start-up state it
  documents, and removes the constant `Encoding::FILESYSTEM` that the alias defines. A
  script that sets `Encoding.default_external` moves the alias again, which the original
  does not: left as it is.
- *`Viewport#disposed?`* belongs to the class surface of RGSS3 (the method list); the
  viewport of RGSS2 has none. The scenario "What is disposed" asks the viewport by a method
  that raises `RGSSError` with `disposed viewport` instead, and the method comes with
  `bind-the-scene-classes-of-rgss3`.
- *`prelude.rb`.* The lines that moved to `prelude_shared.rb` were its last ones; they are
  replaced by a comment of two lines, and no line before them moved.
- *Another target.* `tests/rgss3.rs` of `crates/vkxp` is empty on another target by its
  `cfg` on the target; no other target is installed on the machine of this change, so the
  scenario was not built there. `vkxp-guest-main-rgss3` and `vkxp-guest-rgss3` were built
  with the `cfg` passed by hand (task 3.2).
- *The banner of `README.md`.* The picture `docs/images/banner.jpg` says that VX Ace is
  planned, and its alternative text says what the picture says. The picture is the owner's;
  a new one is a follow-up.

### The release archive (task 6.1)

`cargo xtask dist` was not run: the static library `libcrypt.a` is not installed on the
machine of this change (`cc -print-file-name=libcrypt.a` prints the bare name). The tests
of `dist` pack a source tree; the list of entries the package test expects gains
`vkxp-guest-rgss3` and `licenses/ruby-1.9.2/`, as the scenario "Package" asks, and keeps
every entry it had.

### The stress step (task 7.1)

`VK_LOADER_DRIVERS_SELECT='*lvp*' VKXP_GC_STRESS=1 cargo test -p vkxp -p vkxp-guest-rgss3
--locked --test rgss3 --test guest_record`, the command of the step `guest-1.9.2`, passed
with nothing left out on 2026-10-13: 17.8 s on the development machine with the build done
before, of which `tests/rgss3.rs` took 15.2 s (5.2 s without the variable). No value was
lost, and `xtask/src/stress.skip` gained no entry.

### The check of the local game (task 7.3)

A copy of `../../games/ace-crysalis` under `target/ace-check/game`, both executables of the
development profile, llvmpipe, run from `target/ace-check`:

```sh
XDG_RUNTIME_DIR= VK_LOADER_DRIVERS_SELECT='*lvp*' ../debug/vkxp game --headless \
    --device llvmpipe --rtp RPGVXAce=../../../../ref/ace/rtp \
    --builtin-scripts ../../../../ref/ace/rpg-rgss3.rb --frames 120 \
    --dump 100=frame.png --verbose
```

`XDG_RUNTIME_DIR` is emptied because the shell of this flow may not write
`/run/user/1000` and a socket below `target/` is longer than a Unix socket path may be; the
program then makes its socket directory in the temporary directory, as the tests do.

- Exit status 1. The run said once that VX Ace is experimental, the guest process attached,
  confined with Landlock, and no frame was dumped: the run ended after one frame, that of
  the report's panel (`frame 100 was to be dumped but the run ended after 1 frame(s)`).
- The archive has 126 entries. The built-in scripts file and the entries `{0000}` to
  `{0123}` ran to their end; `{0124}`, `Main`, called `rgss_main`, and the scene manager
  created the game objects through `DataManager`, where `Game_Screen` raised. The last
  entry, without a name or source, did not run.
- The report, as the guest wrote it to the error stream and the host showed it:

  ```
  Script 'Game_Screen' line 49: ArgumentError occurred.

  wrong number of arguments (0 for 3)
  ```

  This is fact 1: `Tone.new` without arguments, the first call that only RGSS3 has.
  Nothing was changed in the classes to get further.

### Follow-ups: the changes that follow

In a suggested order. The first four are what the main test game needs for its title
screen (fact 1); 1 to 7 need nothing of this change and can run beside it, because the
classes are tested in `vkxp-binding-rgss3`.

| # | Change | What it binds | Measured source | Core API it calls | Depends on |
|---|---|---|---|---|---|
| 1 | `bind-the-values-of-rgss3` | `Color.new`, `Tone.new`, `Rect.new` without arguments; `set` with one object of the class; `Rect#initialize` and `set` with any count; `==` with another class; `_load` of a short string; the wording `can't convert` and the count messages | API report 0, 0.2, 2.4; the method list | `vkxp_core::{Color, Tone, Rect, Table}`, nothing new | nothing |
| 2 | `bind-the-window-of-rgss3` | `Window.new(x, y, width, height)` and without arguments, `move`, `open?`, `close?`, `tone`, `padding`, `padding_bottom`, `arrows_visible`, the defaults | `ace-v1-window-REPORT.md`; the method list | `Scene::create_window_rgss3`, `WindowCall` | 1 |
| 3 | `bind-the-scene-classes-of-rgss3` | `Viewport.new` without arguments, `Plane#viewport` and `viewport=`, the scene of RGSS3 (a turned sprite in a viewport, viewports across `resize_screen`, the frozen picture, `snap_to_bitmap` at brightness 0) | sprite report 1 to 5 | `Rules::Rgss3` and the scene of RGSS3 of `draw-sprites-and-snapshots-as-rgss3` | that change merged |
| 4 | `bind-audio-of-rgss3` | `bgm_play` and `bgs_play` with `pos`, `bgm_pos`, `bgs_pos`, `setup_midi`, the lookup order of extensions, the fade-in after an ME | API report 6 | `AudioCommand` with a start position (protocol), `vkxp-audio` | nothing |
| 5 | `bind-input-of-rgss3` | Symbols and the Integers of RGSS2, `false` for anything else, the constants as Symbols, a key seen at shown frames | sprite report 6; the method list | the input rules of RGSS3 of `draw-sprites-and-snapshots-as-rgss3` | that change merged |
| 6 | `bind-font-and-text-of-rgss3` | `Font` with `outline` and `out_color`, the defaults, fonts of `<RTP>/Fonts` and `<game>/Fonts`, `draw_text` and `text_size` by the rules of RGSS3, the glyph rule and `RGSS_VERSION` by the DLL of the game, the largest bitmap | `ace-v1-bitmap-REPORT.md`; `docs/decisions.md`, "Text and bitmaps of RGSS3" | `FontDefaults::rgss3()`, `TextRules::Rgss3`, `squeeze_width_rgss3`, `LibraryVersion` | 1 |
| 7 | `bind-the-tilemap-of-rgss3` | `Tilemap#flags`, the four layers, visibility | `ace-v1-tilemap-REPORT.md` | `Scene::create_tilemap_rgss3` | nothing |
| 8 | `run-the-oracle-on-vx-ace-games` | `cargo xtask oracle` for RGSS3: a probe in the syntax of Ruby 1.9, the registry key `RGSS3\RTP`, the DLL in `System/`, a window that changes its size, `SceneManager.scene`, keys set one call early. The evidence of A1 needs it | V0 report 1 to 7 and "Differences from VX", point 11 | none | this change; 1 to 6 for A1 |
| 9 | `match-the-endings-and-loading-of-rgss3` | the first five items of the list above | API report 1.1, 1.2, 1.4, 1.6, 2.1, 2.2 | none | this change |
| 10 | `emulate-win32api-and-dl-over-ruby-1-9-2` | `Win32API` as RGSS3 has it, `DL` as far as the corpus needs it | API report 5.8, 5.9 | none | this change |
| 11 | `read-strings-in-other-encodings` | arguments in UTF-16LE and the dummy encodings, the encodings made at first use | API report 3.1 to 3.3; the study, work package H | none | 6 |
| 12 | `intern-the-names-of-rgss3` | the names interned at first use, the start-up table of 2170 names | API report 4.1; V0 report 8 | none | a game that shows a need |
| 13 | `play-movies-of-rgss3` | `Graphics.play_movie` | sprite report 5 | to be decided | 3 |
