# Tasks

## 1. Package

- [x] 1.1 `cargo xtask dist` for the host's target, x86_64 or aarch64 Linux. Verify: scenarios "Package" and "Package on aarch64" (unit tests of the name for both arches; `cargo xtask dist` on this x86_64 machine produces the x86_64 archive and `sha256sum -c` passes). If the Jetson `duck-stable` is reachable by ssh, also run `cargo xtask dist` there and record the result; otherwise say so.
  - 2026-10-23: the unit tests of `dist` pass, the archive test for both arches. On the
    development machine (x86_64, Fedora 44, whose `libcrypt.a` came from the signed
    `libxcrypt-static` package through `LIBRARY_PATH`, as there was no root to install it)
    `cargo xtask dist` wrote `vkrgss-0.3.1-x86_64-unknown-linux-gnu.tar.gz` with the files of
    the list, and `sha256sum -c` passed. On the Jetson `duck-stable` (Ubuntu 24.04.5, glibc
    2.39, `CC=gcc-13`) it wrote `vkrgss-0.3.1-aarch64-unknown-linux-gnu.tar.gz` with the same
    files, `sha256sum -c` passed, and the unpacked `vkrgss --version` printed `vkrgss 0.3.1`.

## 2. Release workflow

- [x] 2.1 Build both packages and publish them with the APK. Verify: scenario "Both packages in a release" by reading the workflow; `actionlint` if installed, else a YAML parse.
  - 2026-10-23: `actionlint` is not installed here; the workflow parses with PyYAML (jobs
    `package`, `apk`, `release`). The test `the_release_carries_the_packages_of_both_linux_targets`
    of `xtask/tests/workflows.rs` reads the scenario from the file, and the other tests of the
    release were moved to the new jobs.

## 3. Documents

- [x] 3.1 `docs/building.md`, `README.md`, `CHANGELOG.md`. Verify: the document tests of `xtask` pass; every link in README.md resolves.
  - 2026-10-23: `doc_links` and `former_name` of `xtask` pass; `docs/releasing.md` is
    updated too (see design.md).

## 4. Checks

- [x] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0; `openspec validate package-linux-aarch64 --strict` passes.
