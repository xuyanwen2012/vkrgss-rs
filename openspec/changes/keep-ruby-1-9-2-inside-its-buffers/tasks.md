# Tasks

## 1. Ruby 1.9.2

- [x] 1.1 Find every place at which the fortified C library ends the process in the suites and correct it by a recorded patch; the manifest and the patch log follow. Verify: the scenarios "The three files" and "Under collector stress" with `CARGO_TARGET_DIR=$PWD/target/fortify CFLAGS='-O2 -D_FORTIFY_SOURCE=3'`; the scenario "The plain build".
- [x] 1.2 `docs/development.md`: how the fortified build is tried. Verify: the section on the checks outside the gate names the command and the reason.

## 2. Ruby 1.8.7

- [x] 2.1 Run `cargo test -p vkxp-ruby-sys --locked` with the same flags in a target directory of its own; record the outcome in `design.md` and fix a check of the same kind by a recorded patch. Verify: the run passes, or what does not is named under "Follow-ups" with its cause.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate keep-ruby-1-9-2-inside-its-buffers --strict` passes.
