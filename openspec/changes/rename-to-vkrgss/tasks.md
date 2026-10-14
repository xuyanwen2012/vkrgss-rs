# Tasks

## 1. Deltas and the lists before the rename

- [x] 1.1 Copy `../drafts/rename-to-vkrgss-tools/` to `local/rename/` and check the delta specs against the main specs of the clone; regenerate them if the main specs moved on since `19e8786` (design decision 9). Keep the output of `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test --workspace --locked -- --list` under `local/rename/` (design, Implementation notes). Verify: `python3 -I local/rename/specdeltas.py . openspec/changes/rename-to-vkrgss local/rename/added --check` exits 0 and `openspec validate rename-to-vkrgss --strict` passes.

## 2. Scripted rename

- [ ] 2.1 `git mv` the 19 directories `crates/vkxp*`, the three `third_party/*/README.vkxp.md` and `xtask/src/oracle/vkxp.rs` to their new names (design decision 1). Verify: `git ls-files | grep -i vkxp | grep -v '^openspec/'` lists only `crates/vkrgss-ruby-sys/compat/vkxp_regex.h`.
- [ ] 2.2 Apply the substitution rule (design decision 2) with `local/rename/rename.py` to every tracked text file except `openspec/**`, `CHANGELOG.md`, `Cargo.lock` and the vendored sources and patches under `third_party/`; restore the kept strings of decision 3 (`b"VKXP"` in `codec.rs`, the three names of the Wine prefix in `xtask/src/oracle/original.rs` and `fonts.rs`); the error text of a foreign stream names "the magic of the protocol". Verify: `git diff --stat` touches no file under `third_party/` but `PATCHES.md` and the three READMEs; `git grep -n -i vkxp -- . ':!openspec' ':!CHANGELOG.md' ':!third_party'` shows only the 23 kept identifiers, the stream magic and the three names of the Wine prefix.
- [ ] 2.3 Rewrite the package names in `Cargo.lock` and format. Verify: `cargo metadata --format-version 1 --locked --no-deps` succeeds and lists `vkrgss` and 18 packages `vkrgss-*` besides `xtask`; `cargo fmt --all --check` passes.
- [ ] 2.4 Build and test what moved. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test --workspace --locked` passes, and its `-- --list` output equals the list of task 1.1 with the substitution applied; the scenarios "Packages", "Executables" and "Version line" of `specs/workspace-structure/spec.md`; `cargo test -p xtask --locked` passes (dependency rules, package metadata with the new repository address, the release package).

## 3. Compatibility with version 0.3.0

- [ ] 3.1 The files under the former name: the user file, the game's file and the default built-in scripts files (design decision 5), with tests of every scenario of "Files under the former name". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss -p vkrgss-guest-main -p vkrgss-guest-main-rgss3 -p vkrgss-guest-rgss3 --locked` passes.
- [ ] 3.2 The four environment variables under the former name (design decision 6), with tests of every scenario of "Environment variables under the former name". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss -p vkrgss-guest -p vkrgss-vk --locked` passes; `git grep -n -E 'VKXP_[A-Z]' -- crates xtask` shows only those four names and the kept macros.
- [ ] 3.3 Recordings with the first line `vkxp-input 1` or `vkxp-input 2` (design decision 4), with tests of the scenarios of "Recordings under the former name". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-host -p vkrgss --locked` passes.

## 4. Check of the former name, banner and documents

- [ ] 4.1 Copy `../../vkxp/target/rename-to-vkrgss/banner.jpg` to `docs/images/banner.jpg` if that file exists and set the banner's alternative text in the README to "vkrgss"; if it does not exist, keep both as they are and record that under "Follow-ups" of `design.md` (design decision 11). Verify: `cmp` of the two files when the source exists, or the follow-up recorded.
- [ ] 4.2 `xtask/tests/former_name.rs` and `xtask/tests/former-name.allow` with the entries of design decision 8, each with its reason. Verify: the scenarios of "The former name only where it is kept"; `cargo test -p xtask --locked --test former_name` passes.
- [ ] 4.3 The documents written by hand: the README (title, the sentence on the former name, badges and links to `vkrgss-rs`), the section of `docs/running.md` for users of 0.3.0, the record in `docs/decisions.md`, the "Unreleased" entry of `CHANGELOG.md` (the new names, the fallbacks and that they end in the following release, recordings, the executables, the tracing targets), the description of the package `vkrgss` and the context of `openspec/config.yaml` naming XP, VX and VX Ace. Verify: the scenario "The former name in the README"; the requirements "Public documents" and "Links of the documents resolve"; `cargo test -p xtask --locked` passes; `.github/workflows/release.yml` reads the version of the package `vkrgss` and names the artifact and the release `vkrgss`.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `python3 -I local/rename/specdeltas.py . openspec/changes/rename-to-vkrgss local/rename/added --check` exits 0, and `openspec validate rename-to-vkrgss --strict` passes.
