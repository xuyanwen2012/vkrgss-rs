# Design

## Context

`crates/vkrgss/src/process.rs` (`RGSS3_GUEST`, `guest_executable`, `guest_executable_of`)
chooses the executable of the guest process: the running program for RGSS1 and RGSS2,
`vkrgss-guest-rgss3` beside it for RGSS3. `crates/vkrgss-guest-rgss3` is a binary of a few
lines over `vkrgss-guest-main-rgss3::guest_process`. `vkrgss-guest-main` is the same source
over Ruby 1.8.7.

## Decisions

- **Name.** `vkrgss-guest-rgss12`: it serves RGSS1 and RGSS2, which share Ruby 1.8.7, and
  sits beside `vkrgss-guest-rgss3`. (A naming choice made while the owner was away; it is
  one constant and one package name to change if the owner prefers another.)
- **Package.** A copy of the shape of `crates/vkrgss-guest-rgss3` (manifest, `main.rs`, the
  `cfg` for a target without the interpreter if `vkrgss-guest-main` has one, the message of
  `not_a_guest` naming XP and VX), depending on `vkrgss-guest-main`. Where `xtask` or tests
  check `vkrgss-guest-rgss3` (dependency lists, version equality, nothing of the host
  linked), check `vkrgss-guest-rgss12` the same way.
- **Program.** `guest_executable_of` returns `vkrgss-guest-rgss12` beside the program for
  RGSS1 and RGSS2. The hidden guest role of `vkrgss` itself is removed if nothing else uses
  it (a guest on a thread does not go through it); if a test or the copier of save states
  needs it, say so here and keep only what is needed.
- **Tests of the program.** The requirement "Tests of the program reach the guest
  executable beside it" already makes the tests of `vkrgss` find `vkrgss-guest-rgss3`; the
  same mechanism finds `vkrgss-guest-rgss12`.
- **Package.** `cargo xtask dist` builds and checks the three executables (static libcrypt
  in each).
- **Save states, sandbox, versions.** Unchanged: they live in `vkrgss-guest` and
  `vkrgss-guest-main`, which the new executable runs exactly as `vkrgss` ran them in the
  guest role. The version statement is the package version, which equals the workspace's.

## Decisions taken during the work

- **The hidden guest role of `vkrgss` stays.** The program never starts itself, but three
  requirements this change does not modify still name the role of `vkrgss` itself: "Guest
  main library" (the program calls the function that makes an executable a guest process
  before it reads its command line), "Refused guest record" (`vkrgss --guest-role <path>`)
  and the scenario "The program given a record of VX Ace" of "An executable runs the
  runtimes of its interpreter". What is kept is that one call at the start of `main`; its
  tests (`crates/vkrgss/tests/guest_record.rs`) stay as they are. Nothing of the process
  that runs a game of XP or VX comes from `vkrgss` any more, which is what the macOS
  prerequisite asks.
- **The error of a missing guest executable** is one message for the three runtimes: it
  names the product of the game, the file, the directory and `cargo build -p <name>`.
- **Tests of the program.** The tests with the guest in a process in `end_to_end.rs` find
  `vkrgss-guest-rgss12` beside the program as the tests of RGSS3 find theirs; `cargo test
  --workspace` (the gate and its part `rest`) builds it first, and a test run without it
  fails with the program's own error, which names the file and the command that builds it.
  No check was added in front of each test.
- **The step `guest` of `cargo xtask stress`** (`xtask/src/stress.rs`, outside the files
  the proposal lists, a few lines) selects `vkrgss-guest-rgss12` beside `vkrgss` and adds
  `--test guest_record`, as the step `guest-1.9.2` does: with `--test end_to_end` alone
  cargo does not build the binary of a package that has no such test target, and the
  tests with the guest in a process of that step need it.
- **Documents outside `docs/`.** `README.md` (its build command and the paragraph on the
  interpreters) and `THIRD-PARTY.md` (the number of executables that link libcrypt
  statically) name the executables too; a player who built with the old command of the
  README would have no guest for a game of XP or VX in a window. Both are edited, a few
  lines each.
- **The check of `cargo xtask dist`** ran on this machine with the static libcrypt of the
  distribution's package `libxcrypt-static`, unpacked below `target/` and named to the
  compiler with `LIBRARY_PATH`, because the package is not installed here; the command
  itself is unchanged in that respect.

## Out of scope

macOS itself; moving the socket code out of `vkrgss-proto` (the audit's item 20 names it,
but it is not needed for the executable split).

## Follow-ups

- The scenario "The stress step of Ruby 1.8.7" of "Tests of the program reach the guest
  executable beside it" (`workspace-structure`, not modified here) runs `cargo test -p
  vkrgss --locked --test end_to_end` "with no guest executable of RGSS3 built". Its tests
  with the guest in a process now need `vkrgss-guest-rgss12`, so on a clean tree that
  command passes only once `vkrgss-guest-rgss12` is built; the stress step itself builds
  it. A later change may name the new command in that scenario.
- Removing the hidden guest role from `vkrgss` altogether needs a change that modifies the
  three requirements named under "The hidden guest role of `vkrgss` stays".
