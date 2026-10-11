# Third-party source

The MIT licence of this repository does not cover the directories below. Each stays under
its own terms.

## Ruby 1.8.7-p374

[`third_party/ruby-1.8.7`](third_party/ruby-1.8.7) holds the part of the Ruby 1.8.7-p374
release that `vkxp-ruby-sys` builds into the embedded interpreter. It stays under Ruby's own
terms: the Ruby licence or the GPL version 2, with the exceptions its `LEGAL` file lists,
such as the LGPL for `regex.c`, which is vendored but no longer built. The texts are
[`COPYING`](third_party/ruby-1.8.7/COPYING), [`GPL`](third_party/ruby-1.8.7/GPL),
[`LGPL`](third_party/ruby-1.8.7/LGPL) and [`LEGAL`](third_party/ruby-1.8.7/LEGAL).

Its [`README.vkxp.md`](third_party/ruby-1.8.7/README.vkxp.md) states the origin and how the
subset was chosen, `MANIFEST.sha256` the upstream hash of every file and
[`PATCHES.md`](third_party/ruby-1.8.7/PATCHES.md) every local change.

## Oniguruma 5.9.6

[`third_party/oniguruma-5.9.6`](third_party/oniguruma-5.9.6) holds the part of the Oniguruma
5.9.6 release that regular expressions of the embedded interpreter use. It is under its BSD
licence; the text is [`COPYING`](third_party/oniguruma-5.9.6/COPYING). Its
[`README.vkxp.md`](third_party/oniguruma-5.9.6/README.vkxp.md) states the origin, and
`MANIFEST.sha256` the upstream hash of every file.

## Rust crates

The Rust dependencies are not bundled. `Cargo.lock` names every crate and its version,
[`deny.toml`](deny.toml) the licences and sources that are allowed, and `cargo xtask audit`
checks the one against the other; see
[Checks outside the gate](docs/development.md#checks-outside-the-gate).
