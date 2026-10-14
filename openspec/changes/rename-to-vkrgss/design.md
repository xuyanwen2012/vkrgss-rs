## Context

The owner decided on 2026-10-14 to rename vkxp to vkrgss: the repository on GitHub becomes
`xuyanwen2012/vkrgss-rs`, the program, the guest executable and the crates `vkrgss*`. Version
0.3.0 was released as vkxp the same day; the rename ships in the next version, which a later
release change sets.

### Inventory (measured at `19e8786`, `main` after `release-0-3-0`)

- 2058 tracked files; 521 of them name `vkxp` in any case outside the archived changes and the
  open change `match-the-endings-and-loading-of-rgss3`. Of those, 67 are vendored sources and
  patches under `third_party/` (kept), 18 are main specs (changed through the deltas of this
  change), 6 are `PATCHES.md` and `README.vkxp.md` of the three vendored trees, and about 430
  are the crates, `xtask`, the documents and the workflows. Outside `openspec/` and
  `third_party/` the name occurs 3359 times, in 341 Rust files among others; 937 of the
  occurrences are Rust paths (`vkxp_core::` and the like).
- 19 crates, each in `crates/<name>`: `vkxp`, `vkxp-proto`, `vkxp-pixel`, `vkxp-core`,
  `vkxp-image`, `vkxp-text`, `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-ruby192-sys`,
  `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-guest`, `vkxp-guest-main`,
  `vkxp-guest-main-rgss3`, `vkxp-guest-rgss3`, `vkxp-compose-ref`, `vkxp-vk`, `vkxp-audio`,
  `vkxp-host`; and `xtask`, which keeps its name. Two executables: `vkxp` and
  `vkxp-guest-rgss3`. Two Cargo `links` keys: `vkxp_ruby`, `vkxp_ruby192`. One `cfg`:
  `vkxp_interpreter_absent`. The dependency keys `vkxp-binding` and `vkxp-ruby-sys` that the
  twin packages map to `vkxp-binding-rgss3` and `vkxp-ruby192-sys`.
- Four tracked paths outside the crate directories whose file names hold the name:
  `third_party/{ruby-1.8.7,ruby-1.9.2,oniguruma-5.9.6}/README.vkxp.md`,
  `xtask/src/oracle/vkxp.rs`; and `crates/vkxp-ruby-sys/compat/vkxp_regex.h`, which the
  vendored sources include by that name.
- 36 environment variables `VKXP_*`:
  - read by the program for its users (4): `VKXP_LOG`, `VKXP_GUEST`, `VKXP_SANDBOX`,
    `VKXP_PRESENT_TIMING`;
  - read by the build scripts for the package (1): `VKXP_STATIC_CRYPT`, set by
    `cargo xtask dist`;
  - read by tests and tools (13): `VKXP_TEST_DEVICE`, `VKXP_GC_STRESS`,
    `VKXP_GC_STRESS_SKIP`, `VKXP_BLESS`, `VKXP_RGSS3_FONTS`, `VKXP_TEXT_SAMPLES`,
    `VKXP_FUZZ_TARGET`, `VKXP_FUZZ_SECONDS`, `VKXP_ORACLE_DLL`, `VKXP_ORACLE_RTP`,
    `VKXP_ORACLE_GAME`, `VKXP_PENDING_TEST`, `VKXP_RUBY192_HASH_SEED_CHILD`;
  - handed from build scripts to tests through `cargo:rustc-env` (18): 13 `VKXP_RUBY_*` and
    5 `VKXP_RUBY192_*`.
  Besides these, `VKXP_*` names Ruby constants of tests (`VKXP_EXTRA`, `VKXP_TWICE`,
  `VKXP_INPUT`, `VKXP_MEASURED`, `VKXP_PRELOAD_PATCH`), include guards of own headers and
  shaders (`VKXP_REGEX_H`, `VKXP_ONIGURUMA_CONFIG_H`, `VKXP_FUZZ_HARNESS_H`,
  `VKXP_PIXEL_GLSL`, `VKXP_RECORDS_GLSL`, `VKXP_TRANSFORM_GLSL`) and five macros of the
  vendored sources (kept, below).
- Paths of users: `$XDG_CONFIG_HOME/vkxp/config.toml`, `$XDG_DATA_HOME/vkxp/` with
  `rpg_builtin.rb`, `rpg_builtin_rgss2.rb` and `rpg_builtin_rgss3.rb`, the game's
  `vkxp.toml`, and the private directory `$XDG_RUNTIME_DIR/vkxp-<pid>-<stamp>/` of the
  socket and the record of a guest process. The package
  `vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz`. The oracle's output layout
  (`<out>/vkxp/game`, `frames/<n>-vkxp.png`), its option `--vkxp-only`, and three names in
  its Wine prefix (`vkxp-oracle-prepared`, `drive_c/vkxp-oracle.reg`, the registry values
  `vkxp oracle font <n> (TrueType)`).
- Formats: the recording's first line `vkxp-input 2 seed=<n>` (format 1 is still read); the
  guest record's first line `vkxp-guest-record <version>`; the guest's greeting
  `vkxp-guest <version>`; the 4 bytes `VKXP` before the protocol version in every encoded
  stream (`PROTOCOL_VERSION` 15).
- Texts: the diagnostic prefix `vkxp: `, the tracing targets `vkxp_*` (crate names), the
  version line `vkxp <version>`, the usage text, the default window title `vkxp` and the demo's
  `vkxp demo`, the Vulkan application name, the thread names `vkxp-audio` and `vkxp-guest`,
  the guest executable's own message. Seen by the games' Ruby: `$0` `"vkxp"` in RGSS1 and
  RGSS2 (and the frame `vkxp:in `eval'` of a backtrace), the host functions `vkxp_*`
  (`vkxp_modal`, `vkxp_locate`, `vkxp_win32api`, `vkxp_load_scripts`, ...), the globals and
  instance variables `$vkxp_*` and `@vkxp_*` of the preludes, and the messages of `DL` and of
  an unemulated `Win32API` function.
- Vendored C: 23 identifiers named by the vendored sources or their patches: the ten hooks
  `ruby_vkxp_gc_count`, `ruby_vkxp_gc_stress`, `ruby_vkxp_gvl_atfork`, `ruby_vkxp_hashseed`,
  `ruby_vkxp_hashseed_fixed`, `ruby_vkxp_lazy_timer_thread`, `ruby_vkxp_mark_game_scripts`,
  `ruby_vkxp_select_symbols`, `ruby_vkxp_stack_size`, `ruby_vkxp_static_encoding`; the
  macros `VKXP_ADDRESS_SANITIZER`, `VKXP_FIRST_PROVISIONAL`, `VKXP_GC_STRESS_DUE`,
  `VKXP_LAST_MEASURED_DEFAULT`, `VKXP_STACK_ADDRESSABLE`; and `vkxp_char_end`,
  `vkxp_gc_stress_every`, `vkxp_gc_stress_left`, `vkxp_measured_id`, `vkxp_next_counter`,
  `vkxp_symbol_table`, `vkxp_symbol_tables`, `vkxp_regex` (the header `vkxp_regex.h`; the
  generated header `vkxp_symbol_table.h`). The vendored sources also hold 67 comments with
  the bare word.
- Specs: 18 of the 25 main specs name vkxp; 99 requirements in 17 of them, and the Purpose of
  six (`embedded-ruby`, `embedded-ruby-1-9-2`, `game-files`, `game-launch`, `oracle-harness`,
  `text-rendering`). Four scenario names hold the word ("Capture in vkxp", "Packed game runs
  in vkxp", "Packed game of RGSS3 runs in vkxp", "Capture in vkxp of RGSS3"), and two
  requirement names ("The Rust API of vkxp-ruby-sys", "The vkxp side of a game of RGSS3").
- GitHub: the repository URL in `Cargo.toml` (`repository`, `homepage`), in a test of
  `xtask/src/deps.rs`, in the README's badges and links, and in the release workflow (the
  archive's name, the release title, the package whose version it reads).
- The banner `docs/images/banner.jpg` shows the text "vkxp" at its top left.

## Goals / Non-Goals

**Goals**

- One name everywhere a user, a developer or a game can see it, and in the code.
- Users of 0.1.0 to 0.3.0 keep their files and settings for one version, with a warning that
  says what to move; their recordings keep replaying.
- A test that keeps the former name from coming back.
- Nothing that a game computes, shows or saves changes, apart from the strings that hold the
  program's name.

**Non-Goals**

- Renaming the repository on GitHub, pushing or tagging (manual steps below).
- Drawing the banner.
- Setting the version or writing the release notes (a release change does that).
- Rewriting history: archived changes, the released sections of the changelog, the tags and
  the release assets of 0.1.0 to 0.3.0.
- Touching a vendored source, a patch or a recorded hash.
- Moving or copying a user's files.

## Decisions

### 1. The names

| Former | New |
|---|---|
| repository `github.com/xuyanwen2012/vkxp` | `github.com/xuyanwen2012/vkrgss-rs` |
| crate `vkxp`, `vkxp-<part>`; directory `crates/vkxp*` | `vkrgss`, `vkrgss-<part>`; `crates/vkrgss*` |
| Rust path `vkxp_<part>::` | `vkrgss_<part>::` |
| executables `vkxp`, `vkxp-guest-rgss3` | `vkrgss`, `vkrgss-guest-rgss3` |
| `links` `vkxp_ruby`, `vkxp_ruby192`; `cfg` `vkxp_interpreter_absent` | `vkrgss_ruby`, `vkrgss_ruby192`; `vkrgss_interpreter_absent` |
| variables `VKXP_<name>` | `VKRGSS_<name>` |
| `$XDG_CONFIG_HOME/vkxp/config.toml`, `$XDG_DATA_HOME/vkxp/`, game `vkxp.toml` | `.../vkrgss/config.toml`, `.../vkrgss/`, `vkrgss.toml` |
| socket directory `vkxp-<pid>-<stamp>` | `vkrgss-<pid>-<stamp>` |
| package `vkxp-<version>-<target>.tar.gz` | `vkrgss-<version>-<target>.tar.gz` |
| prefix `vkxp: `, version line `vkxp <v>`, title `vkxp` | `vkrgss: `, `vkrgss <v>`, `vkrgss` |
| recording `vkxp-input 2 seed=`, record `vkxp-guest-record`, greeting `vkxp-guest` | `vkrgss-input 2 seed=`, `vkrgss-guest-record`, `vkrgss-guest` |
| Ruby: `$0` `"vkxp"` (RGSS1, RGSS2), `vkxp_*`, `$vkxp_*`, `@vkxp_*` | `"vkrgss"`, `vkrgss_*`, `$vkrgss_*`, `@vkrgss_*` |
| oracle `--vkxp-only`, `<out>/vkxp/`, `<n>-vkxp.png`, `Side::Vkxp` | `--vkrgss-only`, `<out>/vkrgss/`, `<n>-vkrgss.png`, `Side::Vkrgss` |
| `third_party/*/README.vkxp.md` | `README.vkrgss.md` |
| `xtask/src/oracle/vkxp.rs` | `xtask/src/oracle/vkrgss.rs` |

The repository carries `-rs`, as the owner chose; crates and executables do not, by the
convention of Rust. `xtask` keeps its name. The description of the package `vkrgss`
("Rust + Vulkan runtime for RPG Maker XP (RGSS1) games") and the context line of
`openspec/config.yaml` ("for RPG Maker XP (RGSS1) games") name XP alone, which is the reason
for the rename; both now name XP, VX and VX Ace.

### 2. The substitution rule

One rule decides every occurrence, in files and in paths:

1. `github.com/xuyanwen2012/vkxp` becomes `github.com/xuyanwen2012/vkrgss-rs` (this covers
   `/actions/...`, `/releases/...` and `.git`).
2. Every word that contains `vkxp` in any case (a maximal run of letters, digits and `_`, so
   `vkxp-core` holds the word `vkxp` and `vkxp_regex.h` the word `vkxp_regex`) is kept when it
   is one of the 23 identifiers of the vendored sources (decision 3) or starts with
   `ruby_vkxp_`; otherwise `vkxp` becomes `vkrgss`, `VKXP` becomes `VKRGSS` and `Vkxp`
   becomes `Vkrgss` inside it.
3. The rule is not applied to what decision 3 keeps whole, and the few kept strings of
   decision 3 that rule 2 would change are restored by hand (the stream magic, the names in
   the oracle's Wine prefix).

The list of the 23 identifiers is not typed by hand: it is every word with `vkxp` other than
the bare word in the files under `third_party/` other than `PATCHES.md` and
`README.vkxp.md`. The same rule generated the delta specs of this change.

### 3. What is kept, and why

| Kept | Reason |
|---|---|
| `openspec/changes/archive/**` | History is not rewritten; an archived change describes the tree of its day. |
| The sections 0.1.0 to 0.3.0 of `CHANGELOG.md` | Those versions were released as vkxp; the release notes on GitHub quote them. |
| Vendored sources and `patches/` under `third_party/` (67 files) | Each patched file must match the patched SHA-256 that `PATCHES.md` records, and each patch must apply to the release. Renaming `ruby_vkxp_*` and the other identifiers means editing about 30 patched files, their patches (including the generated `parse.c` ones, whose hunks depend on offsets) and their hashes, for names no user sees. The bare word in their comments stays for the same reason. |
| The 23 identifiers wherever they appear | The Rust declarations (`ffi.rs`, `signals.rs`), the build scripts (`-DVKXP_ADDRESS_SANITIZER`, the generated `vkxp_symbol_table.h`), the compat header `vkxp_regex.h` and the documents must name what the vendored code defines. |
| The 4 bytes `VKXP` at the start of an encoded stream | Not a name anyone reads: a constant that guest and host of one build compare. Changing it would change no behaviour a user can observe; keeping it leaves the protocol as it is, so `PROTOCOL_VERSION` stays 15. The error text that names it says "the magic of the protocol" instead of "the vkxp magic". |
| `vkxp-oracle-prepared`, `drive_c/vkxp-oracle.reg`, `vkxp oracle font <n> (TrueType)` | They live inside the oracle's Wine prefix, which `cargo xtask oracle` reuses across runs. New names would make a reused prefix be prepared again and hold each shared font twice in the font registry, under the former and the new value name, which can change what GDI enumerates in a measurement. They are never shown. |
| Scenario names in the specs that hold `vkxp` (4) | The validator matches a MODIFIED requirement's scenarios by name and refuses a block that drops one; there is no way to rename a scenario. |
| Readers of the former names (decisions 5 and 6) | Compatibility. |

Not kept: the prose of `PATCHES.md` (it names crate paths that move, such as
`crates/vkxp-ruby-sys/data/symbols.startup.txt`, and the program; its fields `File`, `Patch`
and `Patched SHA-256` hold no `vkxp`), and `README.vkxp.md`, which is renamed with the
`OWN_FILES` lists of the manifest tests.

### 4. Formats and the protocol

- Recordings: written with `vkrgss-input 2 seed=<n>`; `vkxp-input 1` and `vkxp-input 2` are
  read as before, with no warning and no end date. The requirement "Raw keys in recordings"
  already keeps recordings of format 1 replaying; a recording reproduces a run and a bug
  report, and reading a second spelling of its first line costs one alternative in the
  parser.
- The guest record and the greeting: renamed, with no compatibility. Both are read only by
  the executable of the same build, which compares the version anyway, and the program
  starts only `vkrgss-guest-rgss3`, so a `vkxp-guest-rgss3` of 0.3.0 left beside it is never
  started.
- The protocol: unchanged (decision 3).
- Save states live in memory (copies of the guest process) and have no file.

### 5. Files under the former name, for one version

The program reads the user file `vkrgss/config.toml` of the configuration directory, the
game's `vkrgss.toml` and the default built-in scripts files under `vkrgss/` of the data
directory. When one of them does not exist and the file at the former place does
(`vkxp/config.toml`, `vkxp.toml`, `vkxp/rpg_builtin*.rb`), it reads that file and writes one
warning event that names both paths and says that the file belongs at the new path. When
both exist, the new one is read and the former one is ignored silently, so a user who keeps
0.3.0 installed beside the new version is not warned on every run. Paths given by
`--config` and `--builtin-scripts` are read as given; `--no-config` reads nothing. A missing
built-in scripts file is reported under the new path only, because that is where it belongs.
Nothing is moved or copied: the game directory may be read-only or shared with another
runtime, and the program writes no file of the user's anywhere else either.

The socket directories `vkxp-*` that a crashed 0.3.0 left in `$XDG_RUNTIME_DIR` are not
swept: that directory is a tmpfs emptied at the end of the session, and the sweep stays
limited to names of the program's own form.

### 6. Environment variables under the former name, for one version

Only the four variables a player sets are read under their former name: `VKXP_LOG`,
`VKXP_GUEST`, `VKXP_SANDBOX`, `VKXP_PRESENT_TIMING`. When the new variable is not set (as its
reader takes "set" today) and the former one is, the former value is used and one warning
event names both. When both are set, the new one wins without a warning. The warnings are
events of level warn, written once the subscriber exists: a former `VKXP_LOG=error` hides
them as it hides every warning.

The other 32 are renamed without a fallback. `VKXP_STATIC_CRYPT` is set by `cargo xtask
dist` itself, and the package check fails when libcrypt ends up shared. The test and tool
variables have callers only in this repository and the owner's notes; an old
`VKXP_TEST_DEVICE` is not silently replaced by llvmpipe either: with the loader restricted to
another driver the tests fail to find llvmpipe. The `cargo:rustc-env` variables are internal.
The filter syntax of `VKRGSS_LOG` names tracing targets, which follow the crate names: a
filter `vkxp_host=trace` taken from `VKXP_LOG` matches nothing now; the changelog and the
section of `docs/running.md` for users of 0.3.0 say so.

### 7. No `vkxp` executable or link

The package and the build hold `vkrgss` and `vkrgss-guest-rgss3` only. A link `vkxp` would
keep the former name alive without an end, print `vkrgss` under the name `vkxp`, and make a
script that calls `vkxp` keep working against the new files and variables without its
author noticing. The package is unpacked into a new directory for each version anyway, so a
user changes the path of the executable at every update; the changelog and the README say
the name changed. A script that calls `vkxp` fails at once with "command not found", which
is the clearest message there is.

### 8. The check of the former name

`xtask/tests/former_name.rs` lists the tracked files with `git ls-files` (the CI checkout and
every clone have `.git`) and skips `openspec/`, whose archived changes are history, whose
main specs change only through deltas and archiving, and whose open changes describe the
rename itself. For every other path and every line of text, it removes each match of the
entries of `xtask/tests/former-name.allow` that apply to the path, and reports the path, or
the path and line number, when `vkxp` in any case remains. An entry is reason lines starting
with `#`, then lines `path <glob>` (a tracked path that may hold the name) or
`text <glob> <regex>` (what may stay in the lines of matching files), as `stress.skip`
pairs a reason with its lines; an entry without a reason fails the test. Initial entries:

- `text third_party/** .*`, with the reason of decision 3; `path` entries for
  `crates/vkrgss-ruby-sys/compat/vkxp_regex.h`. `README.vkxp.md` is not allowed.
- `text ** \b(ruby_vkxp_\w+|<the other 22 identifiers>)\b`.
- `text CHANGELOG.md .*`: the released sections, and the entry of the rename, which names
  what changed.
- The readers of decisions 4 to 6 and their tests: the four variable names in the files that
  read them, `vkxp`, `vkxp.toml` and `vkxp/` as path parts in the configuration and built-in
  scripts code, `vkxp-input` in `crates/vkrgss-host/src/record.rs` and the tests of
  recordings.
- `b"VKXP"` and `"VKXP"` in `crates/vkrgss-proto/src/codec.rs`; the three names of the
  Wine prefix in `xtask/src/oracle/original.rs` and `fonts.rs`.
- Documents: the README's sentence on the former name (and the banner's alternative text
  while the banner shows `vkxp`), the section of `docs/running.md` for users of 0.3.0, the
  record of the rename in `docs/decisions.md`.
- `xtask/tests/former_name.rs` and `xtask/tests/former-name.allow` themselves.

The check stays after the fallbacks are removed: agents copy text from archived changes, and
the test catches a former name that comes back with it.

### 9. Specs: generated MODIFIED requirements

A single ADDED requirement "Name of the project" with a sentence that older scenarios read
with the new name would pass the validator, but it is not clear: AGENTS.md makes the
scenarios the acceptance criteria for the author and the reviewer, the scenarios would keep
quoting `vkxp-core`, `VKXP_LOG` and `vkxp: ` while the code has other names, and every later
change that touches such a requirement would have to translate. So each of the 99
requirements whose text names vkxp is MODIFIED with the whole block, the name substituted by
the rule of decision 2 and nothing else changed; scenario names stay (decision 3); the two
requirement names that hold `vkxp` are RENAMED. Five requirements are ADDED: "Name of the
project" and "The former name only where it is kept" (`workspace-structure`), "Files under
the former name" and "Environment variables under the former name" (`game-launch`),
"Recordings under the former name" (`host-session`).

The MODIFIED blocks are generated, not written: `rename-to-vkrgss-tools/specdeltas.py`
(kept beside this change's draft, outside the repository; the flow copies it to `local/`)
writes them from `openspec/specs` and the hand-written ADDED parts, and with `--check` fails
when a delta differs from what the main specs give. A MODIFIED requirement replaces the whole
block at archive, so a delta made from older main specs would undo what a change archived in
between added; the check, run before the flow starts and again before archiving, prevents
that.

The Purpose of six specs names vkxp, and the delta format has no operation for a Purpose:
at archive, the owner's agent applies the rule to those six lines (decision 10, step 2).

### 10. The GitHub side: manual steps, in order, at release time

Not part of the flow. The files say the new address after the flow; nothing is pushed until
the repository has its new name, so no pushed README points at a repository that does not
exist. GitHub redirects the former web, git and API addresses to the renamed repository as
long as no new repository named `vkxp` is created under the account.

1. Merge the flow's branch into `main` locally; the gate passes there
   (`VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate`).
2. Run `specdeltas.py --check` against `main`, archive the change, apply the rule of
   decision 2 to the Purpose lines of the six specs, and confirm that
   `grep -rni vkxp openspec/specs` shows only the four scenario names, the ADDED
   requirements on the former name and the kept identifiers; `openspec validate --all
   --strict` passes.
3. Ask the owner for an explicit go-ahead to rename the repository now. Without it, stop
   here and push nothing.
4. `gh repo rename vkrgss-rs --repo xuyanwen2012/vkxp --yes`; check that
   `gh repo view xuyanwen2012/vkrgss-rs` answers and that `https://github.com/xuyanwen2012/vkxp`
   redirects. Look at the description and the homepage (`gh repo view --json
   description,homepageUrl`) and change those that name vkxp (`gh repo edit`).
5. `git remote set-url origin` to the new address in the main checkout.
6. Push `main`; wait for the CI workflow to pass under the new name (the README's badges
   resolve then).
7. When the release change of the next version has set the version and its changelog
   section: tag `v<version>` on `main` and push the tag. The release workflow publishes
   `vkrgss-<version>-x86_64-unknown-linux-gnu.tar.gz` and its checksum under the title
   `vkrgss <version>`; check that the unpacked `vkrgss --version` prints `vkrgss <version>`.
   The assets of 0.1.0 to 0.3.0 keep their names.

### 11. The banner

The banner shows "vkxp" as text and was made by the owner. It needs a version with "vkrgss"
in the same place, made by the owner's agent outside the flow; the flow does not draw. If
`../../vkxp/target/rename-to-vkrgss/banner.jpg` (beside the main checkout, outside the
clone) exists when the flow reaches task 4.1, the flow copies it to `docs/images/banner.jpg`
and the alternative text of the README says "vkrgss". Otherwise the old banner stays, its
alternative text keeps "vkxp" because that is what the picture shows, the allow-list keeps
that line, and a follow-up records it.

### 12. Order with other changes

The rename touches almost every file. It runs when no other change is open: after
`match-the-endings-and-loading-of-rgss3` is archived (it adds `vkxp_exit_now` and
`vkxp_label_utf8` and a requirement that keeps `$0` as `"vkxp"` for RGSS1 and RGSS2, which the
regenerated deltas then cover), and before the drafts that wait outside the repository
(`match-the-reset-key-of-rgss3`) are started; those drafts are rewritten by the rule before
their flows.

### 13. Names the games see

`$0` of RGSS1 and RGSS2 becomes `"vkrgss"`; it was `"vkxp"`, which is not the original's
value either (the open change measures `false` for RGSS3 only). The host functions and
prelude variables are renamed. Symbol numbers do not move: the measured tables of RGSS1 and
RGSS2 hold none of these names, names outside the tables are numbered in the order they are
interned, and the rename maps names one to one without changing that order or making two
names equal. Save files hold no name of the program. The symbol and save tests of the gate
confirm it.

## Implementation notes

- Start: copy `../drafts/rename-to-vkrgss-tools/` to `local/rename/`, run
  `python3 -I local/rename/specdeltas.py . openspec/changes/rename-to-vkrgss
  local/rename/added --check`; if it fails because the main specs moved on, regenerate
  without `--check` and commit the deltas in group 1. Keep the lists of
  `cargo test --workspace --locked -- --list` (loader restricted) under `local/rename/` as
  they are before anything moves.
- The scripted rename (`local/rename/rename.py`, not committed): `git mv` each
  `crates/vkxp*` directory to its new name, the three `README.vkxp.md` and
  `xtask/src/oracle/vkxp.rs`; then apply decision 2 to the text of every tracked file except
  `openspec/**`, `CHANGELOG.md`, `Cargo.lock`, binary files and the vendored sources and
  patches under `third_party/`; then restore the kept strings of decision 3 that rule 2
  changed (`codec.rs`, `original.rs`, `fonts.rs`). Run it once on a clean tree; the diff of
  that commit is mechanical and is reviewed as such, the hand edits come in later groups.
- `Cargo.lock`: `cargo metadata --format-version 1 > /dev/null` (or `cargo check
  --workspace`) without `--locked` rewrites the package names; the gate then runs with
  `--locked`.
- `cargo fmt --all` after the substitution: lines grow by two characters per occurrence.
- As implemented, two kept strings beyond the list of decision 3, for the reasons given
  there: the prefix `vkxp-oracle-` of the font files that `xtask/src/oracle/fonts.rs` puts
  into the font directory of the reused Wine prefix (a new prefix would leave the files of an
  earlier run there, which GDI would enumerate beside the new ones; the registry values of
  decision 3 name these files), and the four bytes `VKXP` in a test of
  `crates/vkrgss-guest/src/hello.rs` that stand for the start of a stream of the protocol.
- `CARGO_BIN_EXE_vkxp` and `CARGO_BIN_EXE_vkxp-guest-rgss3` in tests become the new names
  through the rule; tests that look for the guest executable beside the program follow
  `RGSS3_GUEST` in `crates/vkrgss/src/process.rs`.
- The readers of decision 6: `VARIABLE` of `crates/vkrgss-guest/src/diagnostics.rs`,
  `GUEST_VARIABLE` and `SANDBOX_VARIABLE` of `crates/vkrgss/src/args.rs`, the check in
  `crates/vkrgss-vk/src/device.rs`. One small function per crate that reads a new and a
  former name and reports which one it used is enough; the warning is written after the
  subscriber is installed.
- The fallbacks of decision 5: `default_user_file` and `GAME_FILE` in
  `crates/vkrgss/src/config.rs`, `builtin_scripts` in `crates/vkrgss-guest-main/src/lib.rs`.
  A run writes the warning about the built-in scripts once, also with a guest process: the
  side that resolves the default today (the guest) writes it, and nothing else resolves it a
  second time.
  - As implemented: the program itself resolves the file too, in `guest_inputs`, before it
    starts a guest process, and the guest process resolved it a second time. The program now
    hands the file it found to the guest process in the guest record, so the default is
    resolved, and the warning written, once, by the program; with the guest on a thread the
    one resolution is that of the program as before.
- `--help`: the usage text names the new paths and variables only.
- Tests of the new requirements use the stand-in directories the tests already use
  (`XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `HOME` set to temporary directories); no test reads
  the real home.
- The test names change only where they contain the name
  (`the_public_items_of_vkxp_ruby192_sys_are_offered` and the like). Compare the lists kept at
  the start, with the rule applied to the earlier list, against the lists at the end; they
  are equal apart from the tests added by this change.
- Stale outputs: after the merge, `target/` of the main checkout still holds `vkxp` and
  `vkxp-guest-rgss3` from earlier builds, which a script can start without noticing. The flow
  works in its own clone; the owner's agent deletes `target/{debug,release}/vkxp*` in the
  main checkout after merging.
- Documents to write by hand, beyond the rule: the README sentence on the former name, the
  section "Coming from vkxp 0.3.0" of `docs/running.md` (the new paths and variables, the
  fallbacks and when they end, recordings, the executable's new name, `VKRGSS_LOG` filters
  with new target names), the record in `docs/decisions.md` (a paragraph in the list of
  changes at its top and one bullet under "Goal and scope"), the "Unreleased" entry of
  `CHANGELOG.md`.

## Follow-ups

- Remove the fallbacks of decisions 5 and 6 in the release after the one that ships the
  rename (a change of its own), and shrink the allow-list; the reading of `vkxp-input`
  recordings stays.
- The value of `$0` in the originals of RGSS1 and RGSS2 is not measured; `"vkrgss"` is a
  placeholder as `"vkxp"` was.
- If the banner was not provided, `docs/images/banner.jpg` still shows "vkxp"; replace it and
  drop the allow-list entry of its alternative text.
- Outside the repository, for the owner's agent: the scripts and notes that call
  `target/release/vkxp`, `--vkxp-only`, read `<n>-vkxp.png` or set `VKXP_*` (the workspace's
  `CLAUDE.md` names `VKXP_TEST_DEVICE=RADV`), the built-in scripts files in
  `~/.local/share/vkxp/`, and whether the main checkout directory `vkxp/` keeps its name.
