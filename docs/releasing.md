# Releasing

A release is a tag `v<version>` on `main`. The workflow
[`release.yml`](../.github/workflows/release.yml) builds the package for it and publishes
the GitHub release.

## Steps

1. Set the version in `Cargo.toml` (`[workspace.package]`, which every crate inherits) and
   run `cargo check --workspace`, which writes it to `Cargo.lock`.
2. Give the version a dated section in [`CHANGELOG.md`](../CHANGELOG.md), headed
   `## [<version>] - <date>`. The workflow uses that section as the notes of the release
   and fails when there is none.
3. Run the gate, described in [development.md](development.md#merge-gate):

   ```sh
   VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate
   ```

4. Commit, and once the commit is on `main` and its CI run has passed, tag it and push the
   tag:

   ```sh
   git tag v<version>
   git push origin v<version>
   ```

## What the workflow publishes

For a tag the workflow checks that the tag is `v` followed by the version of the workspace,
runs `cargo xtask dist` on Ubuntu 24.04 and creates the GitHub release with two files:

| File | Contents |
|---|---|
| `vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz` | one directory of the same name with the executable `vkxp`, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md` and, under `licenses/`, the licence texts of the bundled Ruby and Oniguruma |
| `vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256` | the SHA-256 checksum of the archive, in the format of `sha256sum` |

Started by hand (`workflow_dispatch`), the workflow only builds the two files and keeps
them as an artifact of the run; nothing is published. That is the way to try the package
before a tag.

The executable is linked against the C library of Ubuntu 24.04, so it needs glibc 2.39 or
newer. At run time it also needs the Vulkan loader with a Vulkan 1.3 driver, ALSA
(`libasound.so.2`), libudev (`libudev.so.1`) and libcrypt (`libcrypt.so.1`). The notes of
the release say so.

## Building the package locally

```sh
cargo xtask dist
```

builds the release executable with `cargo build --release --locked -p vkxp` and writes the
same two files under `target/dist/`. It needs `tar`, `gzip` and `sha256sum`, and fails
with the name of a file of the package that is missing. A package built this way is linked
against the libraries of the machine that built it.

## Checking a package

```sh
sha256sum -c vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf vkxp-<version>-x86_64-unknown-linux-gnu.tar.gz
vkxp-<version>-x86_64-unknown-linux-gnu/vkxp --version
```

The first command prints `OK` for the archive and the last one `vkxp <version>`.
