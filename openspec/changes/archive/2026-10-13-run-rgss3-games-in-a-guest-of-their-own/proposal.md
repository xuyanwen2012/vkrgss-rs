# Run RGSS3 games in a guest of their own

## Why

Everything a game of RPG Maker VX Ace needs below its scripts is on `main`: the interpreter
(`vkxp-ruby192-sys`), the RGSS classes compiled over it (`vkxp-binding-rgss3`), the guest
role as a library, the archive `Game.rgss3a` and the built-in scripts file of RGSS3. No
script of RGSS3 has run in vkxp yet: the program opens the files of such a game and then
refuses it. The two interpreters cannot be linked into one program, so the game needs the
guest executable that decision 2 of `plan-vx-ace` names, and the runtime has to start it as
the original starts a game: the built-in scripts, `$RGSS_SCRIPTS`, each script evaluated as
`{NNNN}`, `RGSSReset`, the error report, `msgbox`, `load_data` that labels strings. Until
that exists nothing of RGSS3 can be run, compared or played.

## What Changes

- A second package of the glue, `vkxp-guest-main-rgss3`, compiles the source of
  `vkxp-guest-main` over `vkxp-binding-rgss3` and `vkxp-ruby192-sys`, defined as the
  binding's twin was. The two interpreter crates gain the two functions the glue needs of
  both: whether a copy of the process would lose script threads, and the restart of the
  thread timer in a copy.
- A new executable, `vkxp-guest-rgss3`: the guest role and that glue, nothing of the host.
  Its `main` is one call.
- The program starts it for a game of RGSS3, found beside its own executable file, and
  ends with an error that names the file and the directory when it is missing. **BREAKING**
  for a run of RGSS3 only: it no longer ends with the message that VX Ace is planned.
- `--guest thread` with a game of RGSS3 is refused with a message that names
  `--guest process`; without a choice a run of RGSS3 uses a process, headless too. Each
  executable refuses a guest record of a runtime its interpreter does not run.
- A game of RGSS3 is prepared, loaded, reset, ended and reported as measured on the
  original: `RGSSReset`, `Graphics.__reset__` and `Audio.__reset__`, `{0001}` in
  backtraces, sources compiled as UTF-8, `load_data` with the relabelling proc,
  `save_data`, `msgbox` and `msgbox_p`, `p` and `print` on the standard output,
  `RGSS_VERSION`, the report `Script '<name>' line <n>: <class> occurred.` and `Unable to
  find file:`. `rgss_main` and `rgss_stop` come from the user's built-in scripts file.
- The functions of Ruby 1.9 that start processes are off by default like the others.
- Save states and the sandbox work for a guest of RGSS3; a state is not saved while a
  script thread is alive, and the guest says so.
- The release archive holds both executables and the licence texts of Ruby 1.9.2.
- VX Ace becomes experimental in the documents and the usage text, and a run of RGSS3
  says so once.
- Tests: generated games of RGSS3, written by test code without an interpreter, run
  headless through both executables; the binding's package over Ruby 1.9.2 tests the
  start-up with a scripted host; `cargo xtask stress` gets a step for the guest of RGSS3.
- Not in this change: the class surface of RGSS3 (`Window`, `Tilemap`, `Font` and text,
  `Sprite`, `Viewport`, `Plane`, `Input`, `Audio`, the forms of `Color`, `Tone` and
  `Rect`). A game sees the classes of RGSS2, so a game made with the standard scripts of
  VX Ace loads its scripts and its database and ends with an error at its first call that
  only RGSS3 has, before its title screen. `design.md` lists the changes that follow, in
  order. Also not: the oracle for RGSS3, `Win32API` and `DL` over Ruby 1.9.2. XP and VX do
  not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: the executable of the guest by runtime and the missing one, the form of
  the guest for RGSS3, a game of VX Ace runs (the refusal goes), the built-in scripts file
  and the archive of RGSS3 in a run, the experimental warning, records of another
  runtime, states and the sandbox with RGSS3.
- `workspace-structure`: the crates `vkxp-guest-main-rgss3` and `vkxp-guest-rgss3` in the
  lists and the dependency rules, the second package of the glue, the guest executable,
  how the tests of the program reach it, the release package with both executables.
- `rgss-runtime`: the preparation, the built-in scripts, the script archive, the endings,
  the error report, the reset, the data files, the message boxes and console output of a
  game of RGSS3, and the process functions of Ruby 1.9.
- `rgss-modules`: `Graphics.__reset__` and `Audio.__reset__`.
- `embedded-ruby-1-9-2`: script threads and copies of the process.
- `supported-runtimes`: VX Ace is experimental; the recorded decisions.

## Impact

- New: `crates/vkxp-guest-main-rgss3` (`Cargo.toml`, `build.rs`), `crates/vkxp-guest-rgss3`
  (`Cargo.toml`, `build.rs`, `src/main.rs`, `tests/guest_record.rs`),
  `crates/vkxp-binding/src/prelude_rgss3.rb`, `prelude_shared.rb` and
  `prelude_process_off_ruby19.rb`, `crates/vkxp/tests/rgss3.rs` and `tests/support/`, test
  files of `crates/vkxp-binding-rgss3/tests/`.
- `crates/vkxp-binding/src`: `runtime.rs` (preparation, loader, reset, report by
  runtime), `kernel.rs`, `graphics.rs`, `audio.rs`, `prelude.rb` (lines moved, numbers
  kept), `testing/host.rs` (the stand-in of RGSS3).
- `crates/vkxp-guest-main/src/lib.rs`: the one `cfg` line, `refuse_rgss3` removed, the
  check of the interpreter, the copier that asks about threads.
- `crates/vkxp/src`: `process.rs` (the executable by runtime), `main.rs` (the form of the
  guest, the warning), `args.rs` (usage text); `crates/vkxp/tests/end_to_end.rs` loses
  the eight tests of the refusal and shares its helpers.
- `crates/vkxp-ruby-sys`, `crates/vkxp-ruby192-sys`: two functions, their tests and
  documentation; the build script of the second reads `VKXP_STATIC_CRYPT`.
- `xtask/src/{deps,dist,stress}.rs` and their tests; `Cargo.toml`, `Cargo.lock`.
- Documents: `README.md`, `AGENTS.md`, `CHANGELOG.md`, `THIRD-PARTY.md`,
  `docs/{running,compatibility,architecture,decisions,development,releasing}.md`.
- The gate: two more packages to lint, build and test; the tests of the program start a
  second executable.
- Not touched: `vkxp-core`, `vkxp-proto` (this change adds no message), the host crates,
  `vkxp-guest`, the classes of the binding, and `third_party/` unless a fault of a copy of
  the process needs a recorded patch.
