# Design

## Decisions

### Metadata

`[workspace.package]` gains `repository = "https://github.com/xuyanwen2012/vkxp"`,
`homepage` with the same value, `readme = "README.md"` where a crate's manifest can name the
root file (otherwise only on `vkxp`), `keywords = ["rpg-maker", "rgss", "vulkan",
"game-engine", "emulation"]` and `categories = ["game-engines", "emulators"]`; the crates
inherit them. A crate without a `description` gets one sentence taken from the crate table of
the README. `publish = false` stays: nothing goes to crates.io.

### Version option

`--version` and `-V` are handled where `--help` is: they print `vkxp <version>` from
`CARGO_PKG_VERSION` to the standard output and the process exits with status 0 without
touching Vulkan, the game directory or the configuration. The usage text lists the option.
The README is not edited by this change (another change rewrites it); the usage text is the
documentation.

### Leftovers

Each of these is looked for and removed where found, and nothing else is refactored:

- `eprintln!`/`println!` that is neither a diagnostic of the program's interface nor in a
  test or example; `dbg!`.
- code that is commented out;
- a dependency in a manifest that no source file of the crate uses (checked by reading
  `use` paths and `cargo build` after removal; `cargo machete` only if it is installed).

What is found and removed is listed in this file under "Found". Finding nothing is a valid
result.

`cargo xtask audit` needs `cargo-deny`, which is not installed here; if it is not available
the audit is not run and that is said under "Found".

## Found

Nothing was removed; the pass changed no file.

- Output: no `dbg!` anywhere. Every `eprintln!`/`println!` outside tests is part of an
  interface: the `vkxp: ...` diagnostics and the usage text of the executable (`main.rs`,
  `process.rs`), the default reporter of a session in `vkxp-host` (`report.rs`), the report of
  a script error that `vkxp-binding` writes to the error stream as `docs/decisions.md` asks
  (`runtime.rs`), the `cargo:` directives of the two build scripts, and the progress and
  result lines of the `xtask` commands. The others are in tests (`vkxp-core`'s tilemap timing
  test, the note of a skipped test in `vkxp`'s `font.rs`).
- Commented-out code: none. The comments that look like code are the C declarations beside
  the layout assertions in `vkxp-vk`'s `present_timing.rs`, which document what is asserted.
- Dependencies: every entry of every manifest is named by a path in the sources it is for
  (`src` for dependencies, `build.rs` for build dependencies, `src`, `tests` and `examples`
  for development dependencies), and every entry of `[workspace.dependencies]` is used by a
  manifest. `libz-sys` in `vkxp-ruby-sys` is used through `extern crate` alone, which is what
  links zlib. The two crates that depend on themselves as a development dependency do so to
  enable their `testing` feature. `Cargo.lock` is unchanged. `cargo machete` is not
  installed and was not run.
- `cargo-deny` is not installed, so `cargo xtask audit` was not run.

## Follow-ups

- `--help` is not an option of its own. The design says the version option is handled "where
  `--help` is", but the program has never had one: `vkxp --help` is an unknown option, which
  prints the usage text to the error stream and exits with status 2. That text lists
  `--version`, so the scenario "Listed in the usage" holds as written, and the version option
  is handled where every other option is, in `args::parse`, with `main` printing and
  returning before the log, the configuration or the game directory are touched. Whether
  `--help` should become an option that prints to the standard output with status 0 is left
  to the owner; it was not built here.
- A test that this change does not touch fails now and then when the whole end-to-end suite
  of `vkxp` runs in parallel: `the_guest_splits_its_frame_time_when_statistics_are_requested`
  finds no `vkxp: guest frame times over` line in the error stream of the run with
  `--guest process`. It failed in 1 of 12 runs of the suite on the commit this branch starts
  from, in 1 of 6 on this branch, and in none of 20 runs of the test alone, so the rows of a
  guest process seem to be lost sometimes when the machine is busy. The first run of the gate
  for this change stopped on it; the gate was run again. Finding the cause is outside this
  change.
