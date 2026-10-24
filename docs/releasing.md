# Releasing

A release is a tag `v<version>` on `main`. The workflow
[`release.yml`](../.github/workflows/release.yml) builds the packages for it and publishes
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
runs `cargo xtask dist` on Ubuntu 24.04 on x86_64 and on aarch64 (the runners `ubuntu-24.04`
and `ubuntu-24.04-arm`) and creates the GitHub release with two files for each `<target>`,
`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, besides the APK
([building.md](building.md#the-release-apk)):

| File | Contents |
|---|---|
| `vkrgss-<version>-<target>.tar.gz` | one directory of the same name with the executable `vkrgss` and beside it the guest executables `vkrgss-guest-rgss12`, which runs the games of RPG Maker XP and VX in a guest process, and `vkrgss-guest-rgss3`, which runs the games of RPG Maker VX Ace, `README.md`, `CHANGELOG.md`, `LICENSE`, `THIRD-PARTY.md` and, under `licenses/`, the licence texts of the bundled Ruby 1.8.7, Ruby 1.9.2 and Oniguruma |
| `vkrgss-<version>-<target>.tar.gz.sha256` | the SHA-256 checksum of the archive, in the format of `sha256sum` |

Started by hand (`workflow_dispatch`), the workflow only builds the files and keeps them as
artifacts of the run; nothing is published. That is the way to try the package
before a tag.

The executables are linked against the C library of Ubuntu 24.04, so they need glibc 2.39
or newer. At run time `vkrgss` also needs the Vulkan loader with a Vulkan 1.3 driver, ALSA
(`libasound.so.2`) and libudev (`libudev.so.1`); the guest executables need none of them,
and have to stay in the directory of `vkrgss`, where the program looks for them. The notes of
the release say so. libcrypt is linked statically into the three, because its shared library has a
different name on Ubuntu (`libcrypt.so.1`) and on Fedora (`libcrypt.so.2`).

## Building the package locally

```sh
cargo xtask dist
```

builds the release executables for the target of the machine it runs on,
`x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu`, in one build with `cargo build
--release --locked -p vkrgss -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3` and writes the two
files of that target under `target/dist/`; on any other machine it fails and says so. It needs
`tar`, `gzip`, `sha256sum` and `readelf`, and fails with the name of a file of the package
that is missing. It sets `VKRGSS_STATIC_CRYPT=1` for the build, so it also needs the static
library `libcrypt.a` (`libcrypt-dev` on Debian and Ubuntu, `libxcrypt-static` on Fedora) and
fails before the build when the C compiler does not find it. After the build it prints the
libraries each executable needs and fails when a libcrypt is among them, naming the
executable. A package built this way is linked against the other libraries of the machine that
built it.

## Checking a package

```sh
sha256sum -c vkrgss-<version>-<target>.tar.gz.sha256
tar -xzf vkrgss-<version>-<target>.tar.gz
vkrgss-<version>-<target>/vkrgss --version
```

The first command prints `OK` for the archive and the last one `vkrgss <version>`. The guest
executable reads no option; started by hand it says what it is and ends.
