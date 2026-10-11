# Tasks

## 1. The manual

- [x] 1.1 Move the sections of the README to the files under `docs/` as in `design.md`, and write `docs/compatibility.md`. Verify: scenario "Manual kept".

## 2. Landing page and the other documents

- [x] 2.1 `docs/images/demo.png` and the new `README.md`. Verify: scenario "Landing page"; the picture is the output of the stated command.
- [x] 2.2 `THIRD-PARTY.md`, `CHANGELOG.md`, `CONTRIBUTING.md`. Verify: each exists and states what `design.md` lists.

## 3. Agent files

- [x] 3.1 `AGENTS.md`, `CLAUDE.md`, `.gitignore`, and `.claude/` and `.agents/` out of the index. Verify: scenario "Machine rules"; `git ls-files .claude .agents` prints nothing and the directories still exist on disk.

## 4. Link check

- [x] 4.1 `xtask/tests/doc_links.rs`. Verify: the scenarios of "Links of the documents resolve"; `cargo test -p xtask --locked` passes.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate publish-documentation --strict` passes.
