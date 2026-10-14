# Tasks

## 1. Program name, title and magic comments

- [x] 1.1 Before anything changes, keep the lists of `cargo test --locked -- --list` named in the Implementation notes under `local/`. `Options::for_rgss(3)` with an empty script name and no `ruby_script` for an empty name in `crates/vkxp-ruby192-sys/src/interp.rs`, with its test and the table "Differences from `vkxp-ruby-sys`" of `lib.rs` brought up to date (design decision 6). Verify: the scenarios of "An interpreter without a script name" and of "Program name of a game of RGSS3"; `cargo test -p vkxp-ruby192-sys -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.
- [x] 1.2 `Untitled` in `guest_inputs` of `vkxp-guest-main` with its unit test (decision 7). Verify: the scenarios of "Title of a game of RGSS3"; `cargo test -p vkxp-guest-main -p vkxp-guest-main-rgss3 --locked` passes.
- [x] 1.3 Tests of the magic comments in `crates/vkxp-binding-rgss3/tests/run_game_rgss3.rs` and `report_rgss3.rs`, and in `crates/vkxp/tests/rgss3.rs` for the statuses (decision 8); a case that fails is fixed in the binding by a check of the runtime. Verify: the scenarios of "Magic comments of the scripts of RGSS3"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp -p vkxp-guest-rgss3 -p vkxp-binding-rgss3 --locked` passes.

## 2. Endings

- [x] 2.1 The modal message `Failed to load script.` in `play` of `runtime.rs` for RGSS3, and the tests of a long report and of the status after an error (decisions 1 to 3). Verify: the scenarios of "Long reports and the status after an error in RGSS3" and of "A script archive of RGSS3 that cannot be loaded"; the scenario "Missing archive" of "Script archive of RGSS3" still passes; the test command of task 1.3.
- [x] 2.2 `at_exit`, `exit!` and `Process.exit!` of RGSS3: the definitions in `prelude_rgss3.rb` and the host function `vkxp_exit_now` (decisions 4 and 5), tested through both executables. Verify: the scenarios of "at_exit and exit! in RGSS3"; the test command of task 1.3; `git diff` against the start of the change touches no file of `crates/vkxp/src`, `crates/vkxp-host` or `crates/vkxp-proto`.

## 3. load_data and save_data

- [ ] 3.1 `GameFiles::data_rgss3` and `VfsError::IsDirectory` in `vkxp-vfs`, with unit tests over an archive and a directory written by the tests (decision 11). Verify: the scenarios of "Data files of RGSS3" of `game-files`; `cargo test -p vkxp-vfs --locked` passes with every name of its kept list.
- [ ] 3.2 `load_data` of RGSS3 as a host function over `data_rgss3`, with a `File` for a loose file, `vkxp_label_utf8`, the name checks, `Errno::EACCES`, and `save_data` with the name checks (decisions 9 and 10); `vkxp_data_bytes` goes. Verify: the scenarios of "Source and errors of load_data in RGSS3" and of "Names of data files in RGSS3"; every scenario of "Data files of RGSS3" still passes; an end-to-end test in `crates/vkxp/tests/rgss3.rs` finds an archive entry without its extension and a loose file beside the entry of the same name; the test command of task 1.3.

## 4. Loading code

- [ ] 4.1 `require_relative`, `Module#autoload` and `Kernel#autoload` in `prelude_rgss3.rb`, files of `require` and `load` compiled as UTF-8 in RGSS3, and `crates/vkxp-binding-rgss3/tests/loading_rgss3.rs` (decision 12). Verify: the scenarios of "Loading code in RGSS3"; the tests of "Loading code" pass unedited over both interpreters; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes.

## 5. Documents

- [ ] 5.1 `docs/compatibility.md` (the section "RPG Maker VX Ace": what is now measured, the known differences with their reasons), `docs/running.md` where it describes the endings of VX Ace, `CHANGELOG.md` under "Unreleased", and the documentation of the changed functions. Verify: the scenarios of "Endings and loading of VX Ace in the documents"; the text names nothing that the code does not do; `cargo test -p xtask --locked` passes.

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, every name of the kept lists is listed again, and `openspec validate match-the-endings-and-loading-of-rgss3 --strict` passes.
