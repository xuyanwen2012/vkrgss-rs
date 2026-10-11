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

To be filled in.

## Follow-ups

- `--help` is not an option of its own. The design says the version option is handled "where
  `--help` is", but the program has never had one: `vkxp --help` is an unknown option, which
  prints the usage text to the error stream and exits with status 2. That text lists
  `--version`, so the scenario "Listed in the usage" holds as written, and the version option
  is handled where every other option is, in `args::parse`, with `main` printing and
  returning before the log, the configuration or the game directory are touched. Whether
  `--help` should become an option that prints to the standard output with status 0 is left
  to the owner; it was not built here.
