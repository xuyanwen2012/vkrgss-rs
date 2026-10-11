# Design

## Decisions

### The switch

`build.rs` of `vkxp-ruby-sys` reads `VKXP_STATIC_CRYPT` (with
`cargo:rerun-if-env-changed`): for `1` it prints `cargo:rustc-link-lib=static=crypt`,
otherwise `cargo:rustc-link-lib=crypt` as today. Any other value than unset, empty, `0` or
`1` stops the build with a message. Development builds and the gate do not set it, so
nothing changes for them and no static library is needed to work on the project.

libxcrypt is under the LGPL 2.1 or later. The package already carries the text of the LGPL
for the interpreter (`licenses/ruby-1.8.7/LGPL`), and the source of everything else that is
linked is public, so relinking stays possible; `THIRD-PARTY.md` gains a paragraph that the
release package links libxcrypt statically, with its licence and where its source is.

### `cargo xtask dist`

- Sets `VKXP_STATIC_CRYPT=1` for its `cargo build`.
- Before building, looks for `libcrypt.a` where the C compiler looks
  (`cc -print-file-name=libcrypt.a` returns a path with a directory when it exists) and
  fails with a message naming the file and the packages (`libcrypt-dev` on Debian and
  Ubuntu, `libxcrypt-static` on Fedora) when it is missing.
- After building, reads the dynamic section of the executable (`readelf -d` if the
  workspace has no ELF reader; a missing tool is reported by name) and fails if a needed
  library's name starts with `libcrypt.so`. The list of needed libraries is printed.

The tests of `dist` that build nothing keep working without the static library; the test
of the needed-library check works on the text of a dynamic section.

### Texts

`docs/releasing.md` and the notes the release workflow appends list the run-time needs
without libcrypt; `docs/building.md` names the static library as a need of `cargo xtask
dist` only.

### What cannot be verified here

The development machine has no `libcrypt.a` (`libxcrypt-static` is not installed), so
`cargo xtask dist` cannot build the package in this run; its early failure with the message
is what can be shown. The owner runs the release workflow by hand afterwards.

## Follow-ups

None yet.
