# Design

## Context

Decided on 2026-10-13 (`docs/decisions.md`, "VX Ace"): a game of RGSS3 runs in a guest
executable of its own, `vkxp-guest-rgss3`, which the program starts instead of itself, and
`vkxp-binding` is compiled a second time as a package of its own over `vkxp-ruby192-sys`.
The change `move-the-guest-role-into-a-library` made the guest role the library `vkxp-guest`
and gathered the glue in `crates/vkxp/src/guest.rs` and `font.rs`. Its first three
follow-ups are what this change settles: how the settings reach a guest that is not the
program, where the glue lives so that two executables have it, and the diagnostics of the
second executable.

What was read for this change:

| Place | What happens today |
|---|---|
| `process.rs`, `GuestProcess::start` | The child is the program itself with `std::env::args_os().skip(1)` and then `--guest-role SEED=SOCKET`. Its environment and working directory are those of the program. |
| `main.rs`, `main` | The child does everything the program does up to the choice of the role: `args::parse`, the level of the diagnostics from `--verbose` and `VKXP_LOG`, `VKXP_GUEST`, `VKXP_SANDBOX`, `config::load` and `config::settle` (both configuration files), `args::check`. Then `guest_settings` copies twelve fields of the options into `GuestSettings`, and `run_guest(game_dir, &settings, &role)` runs. |
| What the child uses of all that | The game directory made absolute, the twelve fields of `GuestSettings`, the seed and the socket of the role, and the two inputs of the diagnostic level. Nothing else: no key map, no device, no dump, no recording, no states schedule. |
| `args.rs`, `config.rs` | Name `vkxp_host::{Filter, InputScript, StateSchedule, KeyMap, GamepadMap}`. |
| `log.rs` | The level, the line format and `install` name `tracing-subscriber` only. `HostReporter` names `vkxp_host::{Reporter, Severity}`, and `EventLines` is the stream of the mixer's reports. |
| `guest.rs`, `font.rs` | Name `vkxp-binding`, `vkxp-ruby-sys`, `vkxp-guest`, `vkxp-vfs`, `vkxp-text`, `vkxp-core`, `vkxp-proto`, `anyhow` and `tracing`; no host type, no `unsafe`. |
| A state (a copy made with `fork`) | Holds the settings, the open game files and the interpreter in its memory. It opens a new connection to the socket whose path it kept, and reads no file and no argument. |
| A reset (F12) | `run_scripts` of the binding starts over from the first preload script. The sources of the preload scripts and the scripts of the game were read once, by `load`, and are kept in memory: a reset reads no file, no argument and no setting, in a process and on a thread alike. |
| The sandbox | Restricts writing and executing, on the interpreter thread, just before the game runs. Reading stays unrestricted. |
| A recording of `--record` | A text file, `vkxp-input 2 seed=<n>`, with a format version of its own. It does not hold the protocol version. What holds it is every framed message and every encoded stream of `vkxp-proto`. |

Four facts follow that the follow-ups did not name:

1. The statement of the version (`vkxp-guest <version>`) is checked by the program when the
   guest attaches, which is after the guest has read its settings, opened the game's files
   and started its interpreter. A record that is read before that needs a check of its own,
   made by the guest.
2. Paths that are not UTF-8 reach the child today, because it derives them as the program
   does: the game directory from the working directory, the paths of a configuration file
   from the directory of that file. `serde` refuses to write such a `PathBuf`, and
   `std::env::args` panics on such an argument. The record has to carry paths as bytes, and
   the guest has to look at its arguments as `OsString`.
3. `docs/decisions.md`, "Process", says `thiserror` in libraries and `anyhow` in the binary.
   The glue uses `anyhow` throughout, and its messages are what the error stream of a run
   shows.
4. The child reads `VKXP_SANDBOX` and `VKXP_LOG` from its environment today. Both are
   settled by the program before the child starts, so they belong in the record with the
   rest.

## Goals / Non-Goals

**Goals:**

- A guest process gets everything from the program in plain data, and runs no parser of the
  command line or of the configuration files.
- The glue is code that a second executable links over another binding package without a
  second copy of the source.
- What a guest executable links reaches no host crate, and a rule of the gate says so.
- XP and VX behave as before in both forms of the guest, and the tests that exist show it.

**Non-Goals:**

- `vkxp-guest-rgss3`, Ruby 1.9.2, the second binding package, the second package of the
  glue, the release archive.
- Refusing `--guest thread` for RGSS3. A game of RGSS3 is still refused before anything
  starts.
- Changing what a setting means, its default, or the order of the places it comes from.
- Handing the guest what the program has already looked up (`Game.ini`, the runtime, the
  directories of the run-time packages). The guest looks them up again, as today. See
  "Follow-ups".
- A new message of the protocol or a new protocol version.
- Keeping the value `SEED=SOCKET` of the hidden option readable.

## Decisions

### The settings travel in a file beside the socket

The program writes one file, the guest record, into the private directory of the socket
(mode 0700, made by `GuestProcess::start`), as `guest.record` beside `guest.sock`, before
it starts the child. The child is started with exactly two arguments: `--guest-role` and
the path of that file.

The three ways compared:

| | (a) File in the socket directory | (b) First framed message on the socket | (c) Everything in the option's value |
|---|---|---|---|
| Order of the start | As today: the guest sets the game up, then connects. | Turned round: the guest has to connect first. Today it connects once the files are open and the interpreter runs, the program gives it 30 seconds for that, and a failure of the set-up is the child's exit before it connects. All of this would change. | As today. |
| Protocol | Untouched. | A new message is a new protocol version, which rejects every stream written with version 12. A frame outside the messages breaks "the messages of the protocol, and nothing else" of requirement "Guest in a process". | Untouched. |
| Diagnostic level | Known before the first event. | Not known until the message is read; the events before that have no level. | Known before the first event. |
| Size | No bound. | No bound in practice. | 128 KiB for one argument on Linux, and half of it once bytes are written as text; over it the start fails with "Argument list too long". |
| Paths that are not UTF-8 | Bytes in a file. | Bytes in a message. | An encoding as text is needed for every byte of the record. |
| The sandbox | The file is read first, long before the guest confines itself, and reading is not restricted anyway. | Nothing to read. | Nothing to read. |
| Copies and resets | Read nothing. | Read nothing; the message is sent on the first connection only, so the connections of copies start differently. | Read nothing. |
| By hand | The file of a run can be copied while the run lasts and given to a guest again. | Needs a program that speaks the message. | The value can be copied from the process list. |
| Left behind | One file, removed with the directory it is in, by the program when the run ends or by the sweep of the next run. | Nothing. | Nothing. |

(a) is taken: it changes neither the order of a start nor the protocol, has no bound, and
carries bytes as they are. Its price is a file that exists for the time of a run, in a
directory that exists for the time of a run already and is cleaned up already.

Also considered: an inherited descriptor (a pipe or a memory file) in place of a path. It
leaves nothing on disk, but it needs `unsafe` on both sides and a descriptor number as the
option's value, and a guest could not be given the record of a run by hand.

Starting a guest by hand needs a program that listens on the socket, whichever way is
taken: the record does not make a guest executable a program of its own. To look at a
guest, a debugger is attached to the child, as today.

### What the record holds

Exactly what the child uses today (see "Context"), and nothing the host alone reads:

| Field | From |
|---|---|
| The seed and the socket (`GuestRole`) | Settled by the program; the socket is made by `GuestProcess::start` |
| The game directory | `absolute` of the directory argument or of `.` |
| `GuestSettings`: the encoding, the runtime named with `--runtime`, the table of run-time packages, the built-in scripts file, the font directories, own font metrics, the preload scripts, debug mode, the process functions, `log_p`, the frame statistics, the sandbox | The settled options: the command line over the environment over the configuration files |
| The diagnostics: the value of `VKXP_LOG` as the program read it, and whether `--verbose` was given | The two inputs of `level` in `log.rs` |

- `GuestSettings` keeps its twelve fields and their meaning. It moves from `guest.rs` to
  `vkxp-guest`, so that the program writes and every guest executable reads one type.
- The program makes every path of the record absolute before it writes it. The functions
  of the glue make them absolute again, which changes nothing, and the record does not
  depend on the working directory of whoever reads it.
- The diagnostics are the two inputs and not a level, so that the child's level is the
  same function of the same values as the program's. The child does not read `VKXP_LOG`.
- The rest of the environment is inherited as today. The glue reads `XDG_DATA_HOME` and
  `HOME` for the default place of the built-in scripts file, and the font engine reads
  what it reads.

### The encoding of the record

```text
line 1   ASCII: "vkxp-guest-record <version>" and a line feed
rest     the record in the postcard encoding, the encoding of the protocol's messages
```

- `<version>` is the package version of the program that wrote the record, under the rule
  of the statement of the version: graphic ASCII, the whole line 64 bytes at most.
- The body is plain data: numbers, booleans, byte strings and lists of them. A path is
  written as its bytes, never as text. The encoding and the runtime, which are
  enumerations of `vkxp-vfs`, are written as small numbers by the module of the record;
  `vkxp-vfs` gets no `serde`.
- There is no format number. The program version is the version of the format: a guest
  reads only a record of its own version, so the body may change with any release.
- The file is created by the program with mode 0600 and must not exist yet. It is written
  whole before the child is started, so the child never sees a part of it.
- No bound on its size is checked.

Considered and not taken: a text of lines `key value`. It could be read and edited by
hand, but it is a format and a parser of our own, with an escape for the bytes of a path,
where the workspace already has an encoding for plain data.

### An executable given `--guest-role`

An executable is a guest process when its first argument is `--guest-role`. It then takes
one more argument, the path of the record, and looks at nothing else of its command line.
It reads its arguments as `OsString`: a socket directory whose path is not UTF-8 is no
longer an error.

A record that cannot be used ends the executable with status 1 and one line on the error
stream, written directly like every fatal message, without the usage text, and before the
subscriber is installed, a game file is opened or a connection is made:

| Case | The line (the intended text) |
|---|---|
| The option is not followed by exactly one argument | `vkxp: --guest-role takes the path of a guest record and nothing else; the program gives it to the guest process it starts` |
| The file cannot be read | `vkxp: cannot read the guest record <path>: <system error>; --guest-role is given by the program to the guest process it starts` |
| No first line of the form, or a body that does not decode | `vkxp: <path> is not a guest record of vkxp <own version>` |
| Another version | `vkxp: the guest record <path> is of version <found>, and this executable is of version <own version>` |

What a user sees who passes the option by hand:

- `vkxp --guest-role 7=/run/user/1000/vkxp-1/guest.sock`, the form of version 0.2.0: the
  value is taken as a path, no such file exists, and the second line of the table is
  written with status 1.
- `vkxp <game directory> --guest-role <anything>`: the option is not the first argument,
  `args.rs` no longer knows it, and the answer is that of any unknown option: `unknown
  option` with the usage text and status 2.

The statement of the version on the socket stays as it is. The two checks are of the two
directions: the guest refuses a record of another program, the program refuses a guest of
another version.

### `vkxp` does the same for its own child

There is one way, and XP and VX go it. The hidden option of `args.rs`, the field
`guest_role` of `Options` and the branch of `main` that follows it go away, with
`GuestRole::parse`, `GuestRole::value` and `RoleError` of `vkxp-guest`. The reasons: the
way of the second executable is then the way that every end-to-end test of the guest in a
process, of the states and of the sandbox goes, and a child that has two ways to its
settings would need both kept the same.

A guest on a thread has no record: it is given the same `GuestSettings` in memory, as
today.

### Order of a guest process

| Today | After |
|---|---|
| `args::parse` of the program's whole command line | the first argument is `--guest-role`; the record is read and its version checked |
| the subscriber from `--verbose` and the child's `VKXP_LOG` | the subscriber from the diagnostics of the record |
| `VKXP_GUEST`, `VKXP_SANDBOX`, both configuration files, `args::check` | nothing |
| `run_guest`: `signals::die_with_parent()`, `guest_inputs`, `guest_config`, `start_interpreter`, `Attached::connect`, the copier, `play_in_process` | the same calls in the same order, with the game directory, the settings and the role of the record |

The exit status of the child follows the rules of today: 0 for a game that ended without a
failure, 1 for one that failed, and 1 after `vkxp: <error and its causes>` for a start that
failed.

### The glue is a library crate, `vkxp-guest-main`

`guest.rs` and `font.rs` move, as they are, into a new library crate on the guest side,
`crates/vkxp-guest-main`. It depends on `vkxp-binding`, `vkxp-ruby-sys`, `vkxp-guest`,
`vkxp-vfs`, `vkxp-text`, `vkxp-core`, `vkxp-proto`, `anyhow` and `tracing`. It forbids
`unsafe`. What it offers (the names are the intended ones and may be adjusted):

```text
guest_process(version: &str) -> Option<ExitCode>
    None when the first argument of the executable is not --guest-role. Otherwise the
    whole guest process: the record, the subscriber, the game, and the status to exit with.
guest_inputs, GuestInputs, guest_on_thread, Outcome, game_succeeded, builtin_scripts_name
    what the program uses for its check before a start and for the guest on a thread
```

What `main` of each executable then contains:

- `vkxp`: as its first statement, `if let Some(status) =
  vkxp_guest_main::guest_process(env!("CARGO_PKG_VERSION")) { return status; }`, and then
  the program as today without the branch of the role. It links the glue for that call and
  for `--guest thread` of RGSS1 and RGSS2.
- `vkxp-guest-rgss3`, when it comes: the same call, and for `None` one line that says the
  executable is started by `vkxp`, with a non-zero status. Nothing else.

The second executable gets the glue over its own binding the way it gets the binding: a
second package whose manifest names the same source (`[lib] path =
"../vkxp-guest-main/src/lib.rs"`) and gives the dependencies their usual names over the
other packages (`vkxp-binding = { package = ... }`, `vkxp-ruby-sys = { package =
"vkxp-ruby192-sys" }`). For that, the glue names the binding and the sys crate only as
`vkxp_binding` and `vkxp_ruby_sys`, which it does today.

Considered and not taken:

- **The glue inside `vkxp-binding`, behind an entry function.** No further package would
  exist twice. But the binding would depend on the font engine, which `font.rs` is outside
  the binding to avoid, and on `fork` and Landlock through `vkxp-guest`; every test binary
  of the binding would link them; and the binding would install a subscriber and exit a
  process. The second package of the glue is a manifest of some twenty lines.
- **The same two files compiled into both executables** (`#[path]`). No crate is added, but
  each binary crate then carries the dependencies of the glue itself, the glue's tests run
  in whichever executable owns the files, and no rule on crates can say that the glue
  reaches no host type, because in `vkxp` it shares a crate with the host half.
- **The record and the subscriber in the glue.** They name no interpreter, so they would
  exist twice for nothing. They go into `vkxp-guest`, which is compiled once.

### The record and the diagnostics are in `vkxp-guest`

`vkxp-guest` gets (intended names):

- `role`: the name of the option, `GuestRole { seed, socket }`, and the function that
  takes the path of the record from the arguments of an executable.
- `settings`: `GuestSettings`.
- `record`: `GuestRecord { role, game_dir, settings, diagnostics }`, writing it, reading
  it, and the error of the four cases above, a `thiserror` type.
- `diagnostics`: what `log.rs` has without a host type: the name of the variable, `Level`,
  `level`, the line format and `install`.

It gains the dependencies `vkxp-vfs` (for `Encoding` and `Runtime` in `GuestSettings`),
`serde` and `postcard` (which it reaches through `vkxp-proto` already) and
`tracing-subscriber`. It still reaches no interpreter and no host crate.

The program installs its own subscriber with the same module, so the lines of the program
and of its guest have one format, as today. `log.rs` of the program keeps `HostReporter`,
`EventLines` and their tests.

### `anyhow` in the glue

The glue keeps `anyhow`. It is the body of an executable's `main`, kept in a library so
that two executables have it: its errors are not matched by a caller, they are written to
the error stream with their causes, and turning them into a `thiserror` type would rewrite
every message of a failed start for no reader. The sentence of `docs/decisions.md`,
"Process", gets the exception: `anyhow` in the binary and in `vkxp-guest-main`. The new
modules of `vkxp-guest` use `thiserror`.

### The dependency rule

`vkxp-guest-main` joins `GUEST_CRATES` in `xtask/src/deps.rs`. The two rules that exist
then say what is wanted: the glue reaches none of `vkxp-vk`, `vkxp-audio` and `vkxp-host`,
directly or transitively, and none of those reaches it. No fifth rule is added.

A guest executable that is not the program links `vkxp-guest-main` and what that reaches,
so the rule is about everything it links. The parsers of the command line and of the
configuration files are modules of the binary crate `vkxp`, which no library can name; the
two scenarios "Arguments of the guest process" and "Configuration files are read once" show
that the child of `vkxp`, which does link them, does not run them.

That the glue has left the program is seen in the manifest of `vkxp`: it no longer
declares `vkxp-ruby-sys` or `vkxp-text`, outside its development dependencies.

### Accepted differences

- With `--verbose`, a run with the guest in a process wrote `read the configuration file
  ...` twice for each file, once by the program and once by the child. It is written once.
  The new end-to-end test uses this.
- With `--verbose` an event is written with its target. The events of the moved code get
  the target of their new crate (`vkxp_guest_main` for `vkxp::guest`). The text of every
  line is unchanged, a directive `vkxp=...` of `VKXP_LOG` still selects them, and no test
  reads a target of these events.
- The child no longer reads `VKXP_LOG`, `VKXP_GUEST` and `VKXP_SANDBOX`. It was started
  with the environment of the program, so the values were the same.
- The arguments of the child in the process list are the option and the path of the
  record, not the command line of the program.

## Implementation notes

- What proves that nothing changed for XP and VX, all in `crates/vkxp/tests/end_to_end.rs`
  and none of them altered: `a_guest_in_a_process_writes_the_frames_of_a_guest_on_a_thread`,
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
  `a_confined_guest_saves_and_loads_states_as_one_that_is_not`. The whole file passes.
- Most tests of the settings (a run-time package, a font directory, a preload script) run
  with the guest on a thread, which is the default of a headless run, and so do not cross
  the record. Scenario "Settings of the run" adds the two that name directories and files,
  one of them by a relative path, with `--guest process`; they reuse the games of
  `a_picture_of_a_run_time_package_loads_when_its_directory_is_given` and
  `a_preload_script_patches_the_game`. The level of the child from `VKXP_LOG` is covered by
  `a_guest_that_is_not_confined_writes_beside_the_game_directory`, which runs with
  `VKXP_LOG=info`; scenario "Level of the guest process" adds `--verbose`.
- Tests are added to `end_to_end.rs` and none of it is changed: `git diff --numstat main
  -- crates/vkxp/tests/end_to_end.rs` shows no removed line.
- Two unit tests go, because what they test goes: the two named
  `the_guest_role_is_a_hidden_option_with_a_seed_and_a_socket`, in `args.rs` and in
  `role.rs` of `vkxp-guest`. The tests of the record take their place. Every other test
  keeps its name; only the crate or module in front of a moved one changes.
- The tests of the refusals start the built program with `--guest-role` and need neither a
  game nor a Vulkan driver. They are a test file of their own in `crates/vkxp/tests`, which
  writes its records with `vkxp-guest`.
- The tests of `font.rs` find the test fonts of `vkxp-text` by
  `CARGO_MANIFEST_DIR/../vkxp-text/tests` and the measured font by `../../local/fonts`.
  Both paths hold for a crate directly under `crates/`.
- The tests of `HostReporter` in `log.rs` write through the line format of the subscriber.
  `vkxp-guest` offers that format so that they stay as they are; `tracing-subscriber` stays
  in the manifest of `vkxp` as far as those tests or the program need it.
- `GuestProcess::start` takes what the record needs beside the socket it makes itself. The
  function that builds the command of the child is separate from the one that starts it,
  so that a unit test reads its arguments.
- Documents: `docs/running.md`, section "Guest process" (the sentence "starts itself again
  for the guest, with its own arguments"); `docs/architecture.md` (the table of crates,
  the lists of guest crates in the rules, the sentence on `unsafe`); `docs/decisions.md`
  (the table of "Crates" and the sentence on `anyhow` under the first "Process"); the
  module documentation of `main.rs`, `process.rs` and `vkxp-guest`.
- The change `vendor-ruby-1-9-2` is open and edits `xtask/src/deps.rs`, `Cargo.toml` and
  `AGENTS.md` too. This change adds one name to `GUEST_CRATES`, one member and one entry of
  `[workspace.dependencies]`, and does not edit `AGENTS.md`.

## Risks

- [A setting is lost or changed on its way through the record] → the record is one type
  with a test that writes and reads every field, and the end-to-end tests of the settings
  run through it whenever the guest is a process.
- [The child behaves differently because a step of its start changed place] → the table of
  the order is the contract; what is left out are parsers whose result the record carries.
- [A guest of another release misreads a record] → it reads the first line before the
  body and refuses another version by name.
- [The record of a run that was killed stays on disk] → it is in the socket directory,
  which the sweep of the next start removes.

## Follow-ups

- **What the program has already looked up.** The program reads `Game.ini`, settles the
  runtime and finds the run-time packages and the built-in scripts file as a check, and the
  child does all of it again from the settings. The record could carry the results. That
  would change where the informational lines of `guest_inputs` come from and what a guest
  executable has to check about the runtime it was built for; decided with
  `vkxp-guest-rgss3`.
- **The second package of the glue** and its name, the rule that nothing depends on both
  binding packages extended to it, and the line of `main` of `vkxp-guest-rgss3` for a start
  without the option: with the change that adds the executable.
- **Moving `Copier` and `TextBackend` below the binding** would shrink the glue. Not needed
  for the second executable.
- **Removing the record once the guest has attached.** It is read once; it is left until
  the directory goes, so that it can be looked at while a run lasts.
- **Requirement "State messages" of `guest-host-protocol`** still says the protocol version
  is 10; the code and `docs/architecture.md` say 12. Not touched here.
