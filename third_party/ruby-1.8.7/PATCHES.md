# Local patches to Ruby 1.8.7-p374

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

## 1. zlib-crc-table-64bit

- File: `ext/zlib/zlib.c`
- Patch: `patches/zlib-crc-table-64bit.patch`
- Patched SHA-256: `0051500303cd272e76cb50cd1bb029716c7ee99074b239f14df8ca0cad75a164`
- Reason: `Zlib.crc_table` read zlib's table of 32-bit values through `const unsigned long *`.
  Where `long` has 64 bits that returns wrong values and reads past the end of the table. The
  pointer type becomes zlib's own `const z_crc_t *`.
