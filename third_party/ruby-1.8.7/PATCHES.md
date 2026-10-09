# Local patches to Ruby 1.8.7-p374

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

No patches yet.
