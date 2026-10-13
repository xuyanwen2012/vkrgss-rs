# Tasks

## 1. The crate and its rules

- [ ] 1.1 Before anything moves, keep the list of tests: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked -- --list` for each test binary, under `local/`. Create `crates/vkxp-guest` with the workspace's metadata and lints, `unsafe` denied at the root, and move `signals.rs`, `sandbox.rs` and the value of `--guest-role` (`GuestRole`, the name of the option, reading and writing `SEED=SOCKET`) into it; `args.rs`, `process.rs` and `main.rs` use them from there; `landlock` leaves the manifest of `vkxp`. Verify: `cargo test -p vkxp-guest --locked` and `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` pass; the names of the tests of both packages are those of the kept list, apart from the module in front of a moved one.
- [ ] 1.2 `xtask/src/deps.rs`: `vkxp-guest` among the guest crates, and the rule that it reaches neither `vkxp-ruby-sys` nor `vkxp-binding`, with unit tests of the rule beside those of the other three. Verify: scenarios "The guest role" and "Interpreter dependency is rejected"; `cargo test -p xtask --locked` and `cargo xtask deps` pass.
- [ ] 1.3 `AGENTS.md` and `docs/decisions.md` (the list of crates that may use `unsafe`, the table of crates), `docs/architecture.md` (the table of crates, the lists of guest crates, the new rule). Verify: scenario "Unsafe elsewhere in the library" (tried once by hand in `sandbox`, not committed); the tests of `xtask` that check the documents pass.

## 2. The copier and the entry functions

- [ ] 2.1 `vkxp-ruby-sys`: the public function that restarts the timer of the interpreter's threads in a copy of the process, with the safety comment of `copier.rs`. Move `copier.rs` into the library with its own result of `park` and the `restart_timer` function in place of the C declaration. Add `Attached` and the two `play` functions as in `design.md`. Verify: `cargo test -p vkxp-guest --locked` passes; `cargo xtask deps` passes.
- [ ] 2.2 Gather the guest's glue of `main.rs` in `guest.rs` with `GuestSettings`, as in `design.md`: `run_guest` and the guest on a thread call the library in the order of the table "Order of a guest process"; the wrapper of the copier implements `Copier` of the binding. Verify: scenario "The program holds no copy of the role" (`grep -rn 'libc::fork\|libc::prctl\|landlock' crates/vkxp/src crates/vkxp/Cargo.toml` finds nothing); `grep -n 'vkxp_host\|crate::args\|crate::config\|crate::process\|crate::log' crates/vkxp/src/guest.rs crates/vkxp/src/font.rs` finds nothing; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes with the test names of the kept list; `git diff main -- crates/vkxp/tests` is empty.

## 3. Version and executable

- [ ] 3.1 The statement of the version: written by `Attached::connect` and by a copy before `Attach`, read and checked by `attached` in `process.rs`; the relay tests that attach by hand write it. Verify: scenarios "Other version", "No statement" and "Statement and message in one write" as unit tests over a pair of sockets; scenario "Same version" with `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` (the tests of states in `end_to_end.rs`, unchanged).
- [ ] 3.2 `GuestProcess::start` takes the runtime and asks the function of `design.md` for the executable. `docs/running.md`, section "Guest process": the statement of the version. Verify: scenario "XP and VX" as a unit test; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate move-the-guest-role-into-a-library --strict` passes; scenario "Nothing else changes" (`git diff main -- crates/vkxp/tests crates/vkxp-binding crates/vkxp-proto crates/vkxp-host` is empty).
