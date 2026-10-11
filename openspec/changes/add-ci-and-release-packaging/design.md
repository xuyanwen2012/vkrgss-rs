# Design

## Decisions

### `cargo xtask dist`

Runs `cargo build --release --locked -p vkxp` and writes, under `target/dist/`:

- `vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz`, holding one directory of the same name
  without the extension with: `vkxp` (the executable, mode 755), `README.md`,
  `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md`, and `licenses/` with the texts the bundled
  source asks to accompany a binary: `ruby-1.8.7/{COPYING,GPL,LGPL,LEGAL}` and
  `oniguruma-5.9.6/COPYING`, copied from `third_party/`;
- `vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256` in the format of `sha256sum`.

The version is the workspace's. The archive is written with the `tar` and `flate2` crates if
the workspace already has them in its lock file, otherwise by running `tar` and `sha256sum`,
which a missing tool reports by name as the other steps of `xtask` do. A file of the list
that is missing fails the command with its name. Entries are sorted and carry a fixed
modification time and owner, so that two runs on the same source give the same archive apart
from the executable.

### `.github/workflows/ci.yml`

On `push` to `main` and on `pull_request`; one job on `ubuntu-24.04`; `permissions:
contents: read`; a concurrency group that cancels a superseded run of the same ref. Steps:
checkout; install the packages `docs/building.md` names for Ubuntu (`build-essential`,
`libcrypt-dev`, `glslc`, `spirv-tools`, `libasound2-dev`, `libudev-dev`,
`mesa-vulkan-drivers`, `vulkan-validationlayers`); the toolchain of `rust-toolchain.toml`
through `rustup show`; a cargo cache (`Swatinem/rust-cache`); the OpenSpec CLI
(`npm install -g @fission-ai/openspec`); then
`VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate`. Third-party actions are pinned to a
major version tag.

### `.github/workflows/release.yml`

On `push` of a tag `v*` and on `workflow_dispatch`. One job on `ubuntu-24.04` with the same
preparation; it checks that the tag, if there is one, equals `v` and the workspace's
version; runs `cargo xtask dist`; uploads the two files as a workflow artifact. Only for a
tag, a second step creates the GitHub release with `gh release create` (`permissions:
contents: write` on that job only), the two files attached, and as notes the section of that
version in `CHANGELOG.md`.

The package is linked against the C library of Ubuntu 24.04 (glibc 2.39) and needs at run
time the Vulkan loader, ALSA and libudev; `docs/releasing.md` and the notes say so.

### Templates

`.github/ISSUE_TEMPLATE/bug_report.md` (vkxp version, distribution, GPU and driver, the game
and its RGSS version, what happened, the output with `--log-level debug` if the option
exists under that name, a reminder not to attach copyrighted game files),
`.github/ISSUE_TEMPLATE/config.yml` (blank issues allowed),
`.github/PULL_REQUEST_TEMPLATE.md` (what and why, the gate command, the OpenSpec change if
behaviour changes).

### `docs/releasing.md`

The steps: version in `Cargo.toml`, dated section in `CHANGELOG.md`, the gate, the tag
`v<version>`, what the workflow publishes, how to check the package (`sha256sum -c`, run
`vkxp --version`).

### What cannot be verified here

The workflows run only on GitHub. Here they are checked for valid YAML (a test in `xtask`
that parses them is not added; `python3 -c 'import yaml'` or an equivalent by hand is
enough) and against this design by the reviewer. The owner corrects what the first real run
shows.

## Follow-ups

None yet.
