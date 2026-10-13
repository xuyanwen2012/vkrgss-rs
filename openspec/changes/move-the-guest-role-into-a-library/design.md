# Design

## Context

Decided on 2026-10-13 (change `plan-vx-ace`, decisions 2, 3, 5 and 10; the study behind
them is outside the repository, `../../ref/tools/ruby192-feasibility/REPORT.md` seen from a
clone under `wt/`, section 7 and work package D): a game of RGSS3 runs in a guest executable
of its own beside `vkxp`; the program starts it instead of itself and checks its version
when it attaches; the binding is compiled a second time as a package of its own over a
second sys crate; "the guest role as a library" can start at once.

What the program crate holds today, read for this change:

| File | What it is | Names |
|---|---|---|
| `process.rs` | The host half only: the socket directory, starting the child, `Attach`, the relay, the states by slot, ending processes. It shares two conventions with the guest: the value `SEED=SOCKET` of `--guest-role` and the first message of a connection. | `vkxp-proto`, `libc` |
| `copier.rs` | Guest: `fork`, the parked copy, the new connection put over the inherited descriptor, `exit_if_copy`. | `vkxp_binding::{Copier, Copied, CopyFailed}`, and the C function `rb_thread_start_timer` of Ruby 1.8.7, declared in the file |
| `sandbox.rs` | Guest: the Landlock rules and their report. | `landlock` |
| `signals.rs` | Guest, both forms: SIGVTALRM blocked and unblocked; guest process: the parent-death signal, SIGCHLD ignored. | `libc` |
| `main.rs`, `run_guest` | The order of a guest process's start. | all of the above |
| `main.rs`, `guest_inputs`, `guest_config`, `start_interpreter` and their helpers | What a guest is set up with. Used by the guest process, by the guest on a thread and, as a check before anything starts, by the host half. | `vkxp_binding::{Config, RunConfig}`, `vkxp_ruby_sys::{Interpreter, SymbolTable}`, `vkxp-vfs` |
| `font.rs` | Guest: the text backend over the font engine. | `vkxp_binding::TextBackend`, `vkxp-text` |
| `args.rs`, `config.rs`, `log.rs` | The command line, the configuration files, the diagnostics. The guest process parses the whole command line of the program and reads the configuration files again. | `vkxp_host::{Filter, InputScript, StateSchedule, KeyMap, GamepadMap, Reporter}` |

Three facts follow that the study did not name:

1. `process.rs` has no guest half to move.
2. A part of the guest role cannot be in a library that links no interpreter: whatever
   names a type or a trait of the binding (`Config`, `Copier`, `TextBackend`, `run_game`)
   or of the sys crate. With two binding packages these are two sets of types.
3. The guest process gets its settings by parsing the program's command line, and that
   parser names types of `vkxp-host`. A guest executable that may not link `vkxp-host`
   cannot use it.

On versions: every framed message already starts with the magic value and the protocol
version of the encoding (`vkxp-proto`, `framing`), and reading one of another version fails
with an error that states both. So a guest of another protocol version is refused today.
What does not exist is a statement of the version of the program.

## Goals / Non-Goals

**Goals:**

- A second executable can link the guest role without linking Ruby 1.8.7, the window,
  Vulkan or audio.
- XP and VX behave as before, in both forms of the guest, and the tests that exist show it.
- The program refuses a guest process of another version.

**Non-Goals:**

- `vkxp-guest-rgss3`, Ruby 1.9.2, a second binding package, the release archive.
- Refusing `--guest thread` for RGSS3, and its headless default: they come with the
  executable. A game of RGSS3 is still refused before anything starts.
- Any change to a message of the protocol or to its version.
- An environment variable or configuration key that names the guest executable. The change
  that adds the second executable decides where it is looked for and whether it can be
  overridden; this change only gives it the one function to change.
- How a guest executable that is not the program gets its settings (fact 3). The guest of
  `vkxp` goes on parsing the program's command line. See "Follow-ups".
- Sharing the glue of "The glue stays in the executable" between two executables. See
  "Follow-ups".
- Moving `Copier` or `TextBackend` out of `vkxp-binding`.

## Decisions

### This is a move, with three additions

Code moves between crates and visibility is widened as far as the move needs. No rule of
the copier, the sandbox or the relay is rewritten, and the order of what a guest does at its
start stays (see "Order of a guest process"). The additions are the entry functions of the
library, the statement of the version, and the function that names the guest executable. A
defect found on the way is listed under "Follow-ups" and left as it is.

### The crate `vkxp-guest`

A library on the guest side. It depends on `vkxp-proto`, `libc`, `landlock`, `tracing` and
`thiserror`, and on no other crate of the workspace. It holds (the names are the intended
ones and may be adjusted):

- `role`: the option `--guest-role`, `GuestRole { seed, socket }`, reading its value and
  writing it (`SEED=SOCKET`), so that the two sides of the convention are one piece of code.
  `args.rs` and `process.rs` use it.
- `hello`: the statement of the version, written by a guest and read by the program.
- `signals`: the four functions of `signals.rs`.
- `sandbox`: `confine`, as it is.
- `copier`: `ForkCopier` and `exit_if_copy`, with the two changes of the next decision.
- At the root, the entry functions of the next decision.

Its errors are a `thiserror` type, as in every library of the workspace. The texts that
reach the error stream ("cannot connect to the socket ... of the parent process", "cannot
attach to the parent process") stay word for word; the type is written so that the
program's `{error:#}` does not print a cause twice.

### The executable hands the game over as a closure

The library never names the binding or the interpreter. The executable gives it the function
that runs the game, and gets back plain things of `vkxp-proto` and of the library itself:

```text
Attached::connect(&role, version) -> Result<Attached, AttachError>
    connects to the socket, states the version, sends Attach
Attached::copier(&self, restart_timer: fn()) -> ForkCopier
    ignores SIGCHLD and makes the copier of this connection
Attached::into_link(self) -> GuestEnd

play_in_process(confine: Option<&Path>, game: impl FnOnce() -> bool) -> bool
    on the interpreter thread: unblocks SIGVTALRM, confines, runs `game`, and ends
    the process there if it is a copy
play_on_thread(game: impl FnOnce() -> R) -> R
    on the interpreter thread: unblocks SIGVTALRM and runs `game`
```

- `ForkCopier::park(slot)` returns the library's own `Parked::WentOn` or
  `Parked::Resumed(GuestEnd)`, or its own failure. The executable wraps the copier in a type
  of about fifteen lines that implements `Copier` of its binding by mapping the two.
- The copier no longer declares `rb_thread_start_timer`. It calls `restart_timer` at the
  place where it called the C function. `vkxp-ruby-sys` gets one public function that does
  what the declaration in `copier.rs` did, with its safety comment, and the program passes
  it. A sys crate of another interpreter offers a function of the same name that does what
  that interpreter needs after a `fork`, which may be nothing.

Considered and not taken:

- **A cargo feature of the library that picks the binding package.** Features of one
  package are unified across a build of the workspace: with both executables in one
  `cargo build --workspace` the library would be compiled with both features and would
  link both interpreters, which is the collision the guest executable exists to avoid.
- **The library compiled twice as two packages over the two binding packages**, as the
  binding itself will be. It works and leaves no glue in the executables, but it makes a
  third doubled package, extends the rule "nothing may depend on both" to it, and puts
  `fork` and Landlock, which have nothing to do with the interpreter, in code that exists
  twice.
- **A trait that the executable implements** (`prepare`, `play`, `restart_timer`). The
  steps of the executable and of the library alternate (see the order below), so the trait
  would have one method per step and say no more than the calls in order do.

The price of the closure: each executable has glue of its own (next decision), and the
order of the calls is in the executable, not enforced by a type. The order is short, it is
stated in the documentation of the crate, and the end-to-end tests of states and of the
sandbox fail when it is wrong.

### The glue stays in the executable, in one module

What names the binding or the sys crate for the guest moves from `main.rs` into a module of
the program, `guest.rs`: `guest_inputs`, `guest_config`, `start_interpreter`,
`symbol_table`, `builtin_scripts`, `builtin_scripts_name`, `archive`, `default_font_names`,
`runtime_of`, `confined`, `game_succeeded`, `run_guest`, the closure of the guest on a
thread, the wrapper of the copier, and their unit tests. `font.rs` stays beside it.

The module and `font.rs` name `vkxp-binding`, `vkxp-ruby-sys`, `vkxp-vfs`, `vkxp-proto`,
`vkxp-guest` and each other. They name neither `vkxp_host` nor `crate::args`,
`crate::config`, `crate::process` or `crate::log`. For that, the functions that took
`&Options` take a plain record, `GuestSettings`, defined in the module, with exactly the
values a guest reads today: the encoding, the runtime named on the command line, the table
of run-time packages, the built-in scripts file, the font directories, own font metrics,
the preload scripts, debug mode, the process functions, `log_p`, the frame statistics and
the sandbox. `main.rs` makes it from the settled options, once, for both halves.

This is what makes the next step possible without touching XP and VX again: the two files
are what a second executable compiles against its own binding package.

### Order of a guest process

As today, step for step:

| Today, in `run_guest` | After |
|---|---|
| `signals::die_with_parent()` | the same function of the library |
| `guest_inputs`, `guest_config` (opens the files, enters the game directory, blocks SIGVTALRM), `start_interpreter` | unchanged, in `guest.rs`; the block is the library's function |
| connect, `Attach` | `Attached::connect`: connect, the statement of the version, `Attach` |
| unless the process functions are allowed: `ignore_children`, `ForkCopier::new` | `Attached::copier` |
| the line that says the guest is not confined | unchanged |
| on the interpreter thread: unblock, `confine`, `run_game`, `exit_if_copy` | `play_in_process` around the closure that calls `run_game` |

The guest on a thread: `play_on_thread` around `run_game`, in the place of the call that
unblocks the signal.

### A guest states its version

- The first bytes of every connection a guest process opens, before `Attach`, are one line
  of ASCII: `vkxp-guest <version>` and a line feed, at most 64 bytes. The version is the
  package version of the executable (`CARGO_PKG_VERSION`), which the executable passes to
  the library. A parked copy states it too: one function of the library opens every
  connection.
- The program reads the line byte by byte, as the copier reads `Resume`, so that nothing of
  the first message is taken, within the time it already gives a process to say what it
  is. A line that is missing, too long or not of this form, or a version other than the
  program's own, is an error that names the version found, when there is one, and the
  program's. For the child this fails the start of the run, and the child is ended as
  after any failed start; for a copy that connects later it is the warning that a failed
  attachment is today.
- It is a line outside the messages, not a field of `Attach`, because a field would be a
  new protocol version, and recordings of version 0.2.0 would be rejected for a check that
  has nothing to do with them. It comes first so that a guest of another release is named
  by its release even when its protocol version differs too.
- The protocol version is not in the line: every message carries it and is refused with
  both versions named (see "Context").

### The executable of the guest

`GuestProcess::start` takes the runtime of the game and asks one function for the
executable to start; the function returns `std::env::current_exe()` for every runtime. The
arguments given to the child are as today. No lookup beside the program, no override.

### `unsafe` and the dependency rules

- The rule on `unsafe` in `AGENTS.md` and `docs/decisions.md` names crates and "platform
  code", which has meant modules of the program crate. The list gets `vkxp-guest`. In the
  crate, `unsafe` is denied at the root and allowed in `copier` and `signals`, the two
  modules that had it. `process.rs` keeps its own, as platform code of the program.
- `xtask/src/deps.rs`: `vkxp-guest` joins `GUEST_CRATES`, so that it may not reach a host
  crate and no host crate may reach it. One rule is added: `vkxp-guest` reaches neither
  `vkxp-ruby-sys` nor `vkxp-binding`, reported with the path like the other three.
- `docs/architecture.md` and `docs/decisions.md`: the row of the crate in the tables, the
  crate in the lists of guest crates, the new rule.

### Accepted difference

With `--verbose` an event is written with its target. The events of the moved code get the
target of their new module (`vkxp_guest::sandbox` for `vkxp::sandbox`, `vkxp::guest` for
`vkxp`). The text of every line is unchanged, a directive `vkxp=...` of `VKXP_LOG` still
selects them, and no test reads a target of these events.

## Implementation notes

- What proves that nothing changed, all in `crates/vkxp/tests/end_to_end.rs` and all
  unchanged by this change: `a_guest_in_a_process_writes_the_frames_of_a_guest_on_a_thread`,
  `a_script_error_of_a_guest_in_a_process_is_reported_as_that_of_a_thread`,
  `a_guest_process_ended_by_a_signal_fails_the_run_naming_the_signal`,
  `the_frame_limit_ends_a_guest_process_too`,
  `a_guest_process_that_does_not_answer_the_quit_request_is_ended`,
  `the_form_of_the_guest_comes_from_the_command_line_the_environment_or_a_file`,
  `the_sandbox_comes_from_the_command_line_the_environment_or_a_file`,
  `a_loaded_state_goes_back_to_the_saved_frame`, `a_state_can_be_loaded_twice`,
  `loading_an_empty_slot_is_reported_and_the_run_goes_on`,
  `no_process_of_a_run_with_states_is_left_when_it_ends`,
  `a_script_error_after_a_load_is_reported_and_fails_the_run`,
  `a_guest_that_may_start_processes_saves_no_state`,
  `a_confined_guest_writes_below_the_game_directory`,
  `a_confined_guest_cannot_write_beside_the_game_directory`,
  `a_guest_that_is_not_confined_writes_beside_the_game_directory` and
  `a_confined_guest_saves_and_loads_states_as_one_that_is_not`; and the unit tests of
  `process.rs`, which stay where they are.
- Unit tests move with their code: the test of `sandbox.rs` to the library, the test of the
  value of `--guest-role` in `args.rs` in two parts (the value to the library, the hidden
  option stays), the tests of `main.rs` that are about the moved functions to `guest.rs`.
  Their names stay; only the module in front of them changes.
- The tests of the statement of the version are unit tests over a pair of sockets, in the
  library for the two directions and in `process.rs` for the refusal. The relay tests of
  `process.rs` that attach by hand (`park`) write the line first.
- `landlock` leaves the manifest of `vkxp`; `libc` stays for `process.rs`.
- `docs/running.md`, section "Guest process": the sentence on what the two processes
  exchange gets the statement of the version.
- Change `plan-vx-ace` rewrites the sections "VX Ace" and "Process" (under "Several
  runtimes") of `docs/decisions.md`. This change edits "Crates" and the list under the
  first "Process" only, so the two do not meet.

## Risks

- [A moved function behaves differently because a step changed place] → the table of the
  order above is the contract of the move; the end-to-end tests of states (the copy, the
  timer, the descriptor) and of the sandbox (confined before the first script) fail when
  it is broken.
- [The line of the version takes bytes of the first message, or the first message is read
  as the line] → it is read byte by byte up to the line feed; a unit test sends the line
  and `Attach` in one write and reads both.
- [The split is not the one the second executable needs] → the glue is left in the program
  on purpose, in two files without host types, and the two open questions are recorded
  below instead of answered by guessing.

## Follow-ups

- **Settings of a guest executable that is not the program.** The child of `vkxp` parses
  the program's whole command line and its configuration files again, with a parser that
  names types of `vkxp-host`. `vkxp-guest-rgss3` cannot link that parser. The change that
  adds it decides how the settled `GuestSettings` reach the child (written by the parent
  into the socket directory, or given in the hidden option), and whether `vkxp` then does
  the same for its own child.
- **The glue in two executables.** `guest.rs` and `font.rs` are needed by the second
  executable against its own binding package: compiled from the same files, or kept in a
  package that exists twice like the binding. Moving `Copier` and `TextBackend` below the
  binding would shrink the glue. Decided with the binding over two back ends.
- **Diagnostics of a second executable.** `log.rs` installs the subscriber and names
  `vkxp_host::Reporter` for the host's reports; the part a guest needs has no host type and
  can move when it is needed.
- **Requirement "State messages" of `guest-host-protocol`** says the protocol version is 10;
  the code and `docs/architecture.md` say 12. Not touched here.
