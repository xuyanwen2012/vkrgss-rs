# Tasks

## 1. Script mode

- [ ] 1.1 `--script FILE [ARG...]` with `--runtime`, the streams, `ARGV`, the exit status and the rules of `require`. Verify: the scenarios of `script-mode` as integration tests that run the built program, for `rgss1` (Ruby 1.8.7) and `rgss3` (Ruby 1.9.2).

## 2. Confinement

- [ ] 2.1 The confinement of a guest, rooted at the script's directory; `--sandbox off`, `--allow-process-functions`. Verify: scenarios "Writes outside the script's directory" and "No program is started" on Linux.

## 3. Documents

- [ ] 3.1 `docs/running.md` (a section on script mode, the option table, `--help`), `CHANGELOG.md`. Verify: the tests that check the documents and the usage text pass.

## 4. Checks

- [ ] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical` (games unchanged), `openspec validate run-a-ruby-script --strict` passes.
