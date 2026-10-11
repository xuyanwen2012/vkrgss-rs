# Oniguruma 5.9.6, vendored for vkxp

This directory is a subset of the Oniguruma 5.9.6 release, kept at its original relative
paths. `vkxp-ruby-sys` compiles it into the regular expression engine of the embedded Ruby
interpreter, as the original runtime's Ruby was built with Oniguruma.

## Origin

- Release: `onig-5.9.6.tar.gz`
- SHA-256 of the tarball:
  `d5642010336a6f68b7f2e34b1f1cb14be333e4d95c2ac02b38c162caf44e47a7`

`MANIFEST.sha256` lists the upstream SHA-256 of every vendored file, in the format of
`sha256sum`. `PATCHES.md` lists the local patches, each with its diff under `patches/`. A test
in `vkxp-ruby-sys` checks the tree against the manifest and the patch list: no file may be
added, removed or changed without a record. The files that vkxp adds (`MANIFEST.sha256`,
`PATCHES.md`, `README.vkxp.md` and `patches/`) are not part of the release.

## Licence

The MIT licence of the vkxp repository does not cover this directory. Oniguruma is under the
BSD licence in `COPYING`; `st.c` and `st.h` are in the public domain, as `st.c` says.
`AUTHORS` is the release's own.

## How the subset was chosen

- The engine: `regcomp.c regenc.c regerror.c regexec.c regext.c regparse.c regsyntax.c
  regtrav.c regversion.c st.c` and the headers `oniguruma.h regenc.h regint.h regparse.h st.h`.
- The GNU-compatible interface that the interpreter's `re.c` calls: `reggnu.c`, `oniggnu.h`.
- The encodings that `$KCODE` can select: `enc/ascii.c`, `enc/utf8.c`, `enc/sjis.c`,
  `enc/euc_jp.c`, and `enc/unicode.c`, which the UTF-8 encoding needs.
- Licence: `COPYING`, `AUTHORS`.

Everything else (the POSIX interface, the other encodings, documentation, samples, tests, the
`configure` machinery, the Windows port) is left out. `config.h`, the one file `configure`
would generate, lives in `crates/vkxp-ruby-sys/config/<target>/oniguruma/`.
