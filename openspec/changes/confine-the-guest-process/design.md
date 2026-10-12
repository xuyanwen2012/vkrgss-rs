# Design

## Context

- `crates/vkxp/src/process.rs` starts the program again as the guest; the child settles its
  configuration from the same arguments, sets the game up and connects to the parent's Unix
  socket. Save states are forks of the guest (`copier.rs`), which inherit what the guest
  restricted itself to, and connect to the same socket by its path.
- The guest reads the game directory, the run-time packages, the built-in scripts file, the
  fonts of the system and the configuration files. It writes where the game's scripts write,
  which is below the game directory (saves), and what options of the run tell it to write
  (the guest's rows of `--frame-stats`).
- Landlock restricts the calling thread and what it starts later. Its file rules do not
  cover connecting to a Unix socket by path. Rules for TCP came with ABI 4. The `landlock`
  crate (by the maintainers of Landlock, MIT or Apache-2.0) offers the rules without
  `unsafe` and a mode in which rights an older kernel does not know are left out.

## Decisions

### What the guest may do

Applied on the thread that will run the interpreter, after the configuration is settled and
before the first script, the preload scripts included:

- read and list: everything;
- write, create, remove, rename, truncate: below the game directory, and writing the
  device `/dev/null`;
- what an option of the run tells the guest process to write is opened before the
  restriction, or the directory it names is granted like the game directory; the
  implementer lists these under "Checked";
- execute: nothing;
- TCP: neither bind nor connect, where the kernel has those rules.

`PR_SET_NO_NEW_PRIVS` is set as Landlock requires. Nothing is restricted for the host
process. No seccomp filter is installed.

### Best effort

The rules are made in the crate's best-effort mode. When the kernel has no Landlock, or the
call fails, the guest reports once at the warning level what is missing and runs
unconfined. When only some rights are missing (an older ABI), it reports at the information
level which.

### The option

`--sandbox on|off`, the environment variable `VKXP_SANDBOX` and the configuration key
`sandbox` (a boolean in the file), in the usual order of precedence; any other value is an
error like the one of `--guest`. The default is on. With `--allow-process-functions` the
guest is not confined, whatever the option says, and the run says so once at the
information level: a program the game starts would inherit the restriction and fail in
ways that look like defects of that program.

A guest on a thread is not confined, because the restriction would bind the thread and not
the files the two sides share; `--sandbox on` with `--guest thread` is accepted and
reported once at the information level.

### Tests

End to end, with generated games in temporary directories, each skipped with a printed
reason when a probe finds that this kernel or this sandboxed test environment has no
Landlock: a script writes a file in the game directory (succeeds); a script writes a file
in a sibling temporary directory (`Errno::EACCES`, the file does not exist); the same with
`--sandbox off` (succeeds); a save and a load of a state under the restriction
(`--states`) give the frames they give without it.

## Risks

- [A game writes outside its directory on purpose] → `--sandbox off`, per game in its
  configuration file.
- [The flow's own sandbox hides Landlock from the tests] → the tests skip there; the owner
  runs them outside at the merge.

## Follow-ups

None yet.

## Checked

- Task 1.1 names `cargo test -p vkxp --locked --lib --bins`. The package has no library
  target, so cargo refuses `--lib` before it builds anything, on the commit before this
  change as well. `cargo test -p vkxp --locked --bins` runs the same unit tests and passes.
- What options tell the guest process to write: only the guest's rows of `--frame-stats`
  and the diagnostics of `--verbose` and `VKXP_LOG`, and both go to the error stream, which
  the guest inherits open. Nothing is opened early and no directory besides the game
  directory is granted. `--record` and `--dump` are written by the host process.
- The rights handled are `Execute` and every right of writing that Landlock ABI 4 knows
  (write, the removals, the creations, `Refer`, `Truncate`), and the two TCP rights. The
  game directory is granted all of them but `Execute`, so a rename between two of its
  directories works. `/dev/null` is granted write and truncate, because Ruby opens a file
  for writing with `O_TRUNC`.
- Landlock ABI 5 added a right for requests to devices and ABI 9 one for connecting to a
  Unix socket by its path. Neither is handled, so both stay as they are: the second is what
  the copies of the guest process need to reach the socket of the program.
- This machine: Linux 7.2.9, Landlock ABI 10; `landlock` 0.4.7 knows up to ABI 9 and
  reports the rules as fully enforced.
- The four end-to-end tests of the restriction ran here and passed; none was skipped. The
  test of "Off" needs no Landlock and always runs.
- `landlock` and the two crates it brings (`enumflags2`, `enumflags2_derive`) are MIT or
  Apache-2.0, which `deny.toml` allows already; it is unchanged. `cargo-deny` is not
  installed here, so `cargo xtask audit` was not run; `cargo xtask deps` passes.
