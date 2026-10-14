## Why

The owner decided on 2026-10-14 to rename the project. "xp" names RPG Maker XP alone, while
the program has run the games of XP, VX and VX Ace (RGSS1, RGSS2 and RGSS3) since version
0.3.0; "rgss" names exactly those three runtimes. The repository becomes `vkrgss-rs` on
GitHub (it is `xuyanwen2012/vkxp` today), and the program and the crates become `vkrgss`:
by the convention of Rust, a crate or an executable carries no `-rs`. Version 0.3.0 was
released as vkxp; the rename ships in the next version, so this change does not set a
version.

## What Changes

- **BREAKING** The 19 crates `vkxp` and `vkxp-*` become `vkrgss` and `vkrgss-*`, their
  directories under `crates/` move with them, and every path in Rust (`vkxp_core::` and the
  like), every Cargo `links` key, the `cfg` `vkxp_interpreter_absent` and the variables that
  build scripts hand to the compiler follow.
- **BREAKING** The executables become `vkrgss` and `vkrgss-guest-rgss3`; the release archive
  becomes `vkrgss-<version>-x86_64-unknown-linux-gnu.tar.gz`. No executable or link keeps
  the name `vkxp`.
- **BREAKING** Everything the program says or writes under its name: the prefix `vkrgss: `
  of the diagnostics, `vkrgss <version>` of `--version`, the usage text, the window title
  and the application name given to Vulkan, the names of its threads, the directory of the
  socket of a guest process, the first lines of the guest record and of the guest's
  greeting, the first line `vkrgss-input 2` of a new recording, and in the games' Ruby `$0`
  of RGSS1 and RGSS2, the frame name in a backtrace and the names of the host functions and
  variables of the preludes.
- **BREAKING** The environment variables `VKXP_*` become `VKRGSS_*` (36 names), the user
  configuration file `$XDG_CONFIG_HOME/vkrgss/config.toml`, the game's file `vkrgss.toml`
  and the default built-in scripts files `$XDG_DATA_HOME/vkrgss/rpg_builtin*.rb`.
- For one version, the program reads a file or one of four environment variables
  (`VKXP_LOG`, `VKXP_GUEST`, `VKXP_SANDBOX`, `VKXP_PRESENT_TIMING`) under the former name
  when the new one is absent, with one warning that names both. Recordings whose first line
  is `vkxp-input 1` or `vkxp-input 2` keep replaying, without a warning and without an end.
- Kept as they are: the archived changes, the sections 0.1.0 to 0.3.0 of `CHANGELOG.md`, the
  vendored sources and patches under `third_party/` with the 23 identifiers they name
  (`ruby_vkxp_*` and others), the four bytes `VKXP` that start a stream of the protocol, and
  three names inside the Wine prefix of the oracle. The scenario names of the specs that
  name vkxp stay, because the validator matches scenarios by name.
- A test of `xtask` with a committed allow-list fails when `vkxp` appears outside what is
  kept.
- Documents: the README (title, banner, badges, one sentence on the former name), `AGENTS.md`,
  `docs/*`, `CONTRIBUTING.md`, `THIRD-PARTY.md`, `LICENSE`, the issue template, the
  workflows, `openspec/config.yaml`, `docs/decisions.md` (a record of the rename), a section
  of `docs/running.md` for users of 0.3.0, and an entry under "Unreleased" of `CHANGELOG.md`.
  The three `README.vkxp.md` of the vendored trees become `README.vkrgss.md`.
- The repository address in every manifest and document becomes
  `https://github.com/xuyanwen2012/vkrgss-rs`. Renaming the repository on GitHub, pushing
  and tagging are done by hand outside the flow, in the order `design.md` gives.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the name of the project, the check of the former name, and the
  crate names, package metadata, release archive and executables of 19 requirements.
- `game-launch`: the files and the four environment variables under the former name, and
  the names in 18 requirements (executables, messages, variables, paths).
- `host-session`: recordings under the former name, and the name in one requirement.
- `binding-over-two-interpreters`, `core-scene`, `embedded-ruby`, `embedded-ruby-1-9-2`,
  `frame-composition`, `oracle-harness`, `rgss-modules`, `rgss-runtime`,
  `rgss-scene-classes`, `rgss-values`, `supported-runtimes`, `text-rendering`,
  `vulkan-compositor`, `vulkan-presentation`: their requirements that name vkxp, its crates
  or its variables, with the new name and nothing else changed (two requirement names
  change: "The Rust API of vkrgss-ruby-sys" and "The vkrgss side of a game of RGSS3").

## Impact

Every crate (19 directories moved), `xtask` (the release package, the dependency rules, the
oracle, the new test of the former name), `Cargo.toml`, `Cargo.lock`, the documents listed
above, `.github/`, `openspec/config.yaml`, `third_party/*/PATCHES.md` and the three READMEs
there. About 435 tracked files change, most of them only by the name. Not touched: the
vendored sources and their patches and hashes, the archived changes, the released sections of
the changelog. Outside the repository (by hand, not in the flow): the repository on GitHub,
the banner picture, and the owner's scripts, notes and installed files that name vkxp.
