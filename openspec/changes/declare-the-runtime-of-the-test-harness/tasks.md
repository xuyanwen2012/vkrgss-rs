# Tasks

## 1. Harness

- [x] 1.1 `start_as` returns the handle; the functions that reach the interpreter become its methods; the fallback is removed; the directory name gains the process id; the panic names both runtimes. Save the lists of tests of both packages (`cargo test -p vkrgss-binding -p vkrgss-binding-rgss3 --locked -- --list`) before starting, under `local/`. Verify: scenarios "No default runtime", "Two runtimes in one process", "Two processes of one test executable" (a unit test of the name function is enough for the last).

## 2. Tests

- [x] 2.1 Move every user of the harness to the handle; files that relied on the fallback declare RGSS1. Verify: scenario "Same tests" against the saved lists; `cargo test -p vkrgss-binding -p vkrgss-binding-rgss3 --locked` passes with the default threads and with `-- --test-threads=1`.

## 3. Documentation

- [x] 3.1 `docs/development.md` (if it describes the harness) and `CHANGELOG.md` under Unreleased. Verify: the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate declare-the-runtime-of-the-test-harness --strict` passes.
