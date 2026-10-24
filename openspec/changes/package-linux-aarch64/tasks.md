# Tasks

## 1. Package

- [ ] 1.1 `cargo xtask dist` for the host's target, x86_64 or aarch64 Linux. Verify: scenarios "Package" and "Package on aarch64" (unit tests of the name for both arches; `cargo xtask dist` on this x86_64 machine produces the x86_64 archive and `sha256sum -c` passes). If the Jetson `duck-stable` is reachable by ssh, also run `cargo xtask dist` there and record the result; otherwise say so.

## 2. Release workflow

- [ ] 2.1 Build both packages and publish them with the APK. Verify: scenario "Both packages in a release" by reading the workflow; `actionlint` if installed, else a YAML parse.

## 3. Documents

- [ ] 3.1 `docs/building.md`, `README.md`, `CHANGELOG.md`. Verify: the document tests of `xtask` pass; every link in README.md resolves.

## 4. Checks

- [ ] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0; `openspec validate package-linux-aarch64 --strict` passes.
