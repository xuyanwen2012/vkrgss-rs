# Design

## Context

- The root `Cargo.toml` has `[workspace.lints]` (`unsafe_op_in_unsafe_fn` denied, `clippy::all`
  and `clippy::undocumented_unsafe_blocks` as warnings, which the gate's `-D warnings` turns
  into errors) and no `[profile.*]` section. `.cargo/config.toml` holds only the `xtask` alias.
- `xtask/src/gate.rs` runs four steps in order (`fmt`, `clippy`, `test`, `openspec`) and stops
  at the first failure, naming it. A `Step` is a name, a program and arguments.
- `cargo test --workspace` includes the upstream suites of the vendored interpreter, which
  run as child processes of a C program built at `-O2` whatever the profile, and the Vulkan
  tests on llvmpipe.
- The modules of most crates are private and their items are re-exported from the crate
  root, so many items are declared `pub` inside private modules without being reachable.
- `docs/decisions.md` says of the CPU reference compositor "It is selectable at run time for
  debugging"; the executable has no such switch and `vkxp-compose-ref` is a development
  dependency of `vkxp-host`. Its crate table gives "configuration" to `vkxp-host`; the
  configuration file is read in `crates/vkxp/src/config.rs`.

## Decisions

- Profiles, all in the root `Cargo.toml`:
  - `[profile.dev]` with `opt-level = 1`, and `[profile.dev.package."*"]` with
    `opt-level = 2`. Debug assertions and overflow checks stay on, as the `dev` profile has
    them. The `test` profile inherits from `dev`, so the gate's tests get both.
  - `[profile.release]` with `lto = "thin"`, `codegen-units = 1` and
    `debug = "line-tables-only"`.
  - `[profile.profiling]` inheriting `release` with `debug = true`.
- The gate's steps become `fmt`, `clippy`, `test`, `doc`, `openspec`. `doc` is
  `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`; a `Step` gains
  the environment it sets. `clippy` and `test` get `--locked`, and so does the `xtask` alias in
  `.cargo/config.toml`: the outer `cargo run` resolves the workspace before `xtask` starts, so
  without it a stale lock file is rewritten before any step looks at it. After each step the
  gate prints its name and duration, and at the end one line per step with its duration.
- What the documentation step reports (broken or ambiguous links, bare URLs, invalid markup)
  is fixed in the comments. A link from public documentation to a private item becomes plain
  code text rather than making the item public.
- `unreachable_pub` (rustc), `clippy::dbg_macro` and `clippy::todo` join the workspace lints as
  warnings. An unreachable `pub` becomes `pub(crate)`, or `pub(super)` where that is what the
  code means. Where an item is deliberately `pub` for a reason the lint cannot see, an
  `#[expect(unreachable_pub, reason = "...")]` at that item is fine.
- `docs/decisions.md`:
  - The sentence on the reference compositor becomes: it is a library used by the tests and
    tools; a switch in the executable that renders with it is reserved, not implemented.
  - In the crate table, "configuration" moves from the `vkxp-host` row to the `vkxp` row
    ("Executable: command line, configuration files").
  - The merge gate line in "Process" names the documentation step.
- `README.md`: the gate table gets the new step and flags; the section on running a game says
  that `cargo build --release -p vkxp` produces the optimised executable.
- The time the gate takes before and after, measured once each on this machine with a warm
  build directory, is written under "Measured" at the end of this file.

## Risks / Trade-offs

- [A test that only passed because of unoptimised code, or a float result that changes] → The
  compiler does not reorder float arithmetic at any level; the gate shows it.
- [`unreachable_pub` touches many files] → The edits are visibility keywords only; this change
  runs alone, before the others of the drive, so nothing conflicts.
- [`--locked` fails when a manifest changes without the lock file] → That is the point; the
  lock file is committed together with the manifest.

## Follow-ups

- Dependency advisories and licences (`cargo deny`), in a later change.
- `missing_docs`, once the number of reports is known.
- The gate fails at the `test` step when it is started with SIGINT ignored, as a tool that
  runs it without a terminal may do. `test_should_propagate_signaled` in
  `third_party/ruby-1.8.7/test/ruby/test_beginendblock.rb` starts a child interpreter that
  sends itself SIGINT and expects the output to end in `Interrupt`; the ignored disposition is
  inherited through every child process, and `install_sighandler` in `signal.c` keeps a
  disposition that is not the default, so the child is not interrupted and prints nothing.
  Measured with `cargo test -p vkxp-ruby-sys --test upstream`, twice each way:
  `env --default-signal=INT` passes (5 of 5) and `env --ignore-signal=INT` fails with
  `<""> expected to be =~ </Interrupt$/>`. The commit before this change (`df34240`) gives the
  same two results, and the change touches neither the test nor the interpreter, so it is not
  a regression of this change. Run the gate from a terminal, or under
  `env --default-signal=INT`. Whether the gate or the test harness should restore the default
  disposition itself is left open; neither is built here.

## Measured

One run each on the development machine, with a warm build directory and the Vulkan tests on
llvmpipe.

- Before the change (four steps, no profiles): 83.4 s in total, of which the test step 82.4 s.
- After the change (five steps, with the profiles): 38.9 s in total, of which the test step
  37.7 s and the documentation step 0.1 s.
