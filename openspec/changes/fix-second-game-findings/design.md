# Design

## Context

- Measured with a probe script in the original runtime (Ruby 1.8.1) and in vkxp; the scenario
  "Measured on the original" lists what the original printed. In vkxp the same script raised
  `TypeError: superclass mismatch for class KA`.
- 1.8.7's `eval.c`, node `NODE_CLASS`: when the constant exists, is a class, and a superclass
  is given, it compares `rb_class_real(RCLASS(klass)->super)` with the given one and raises on
  a difference. Ruby 1.8.1 at the same place jumps to the code that defines a new class
  (`override_class`) instead of raising.
- `crates/vkxp-binding/src/win32api.rs` holds the table of emulated functions keyed by library
  and function name; unknown pairs return 0 and are reported once.
- `xtask/src/oracle` reads the script archive as a `Marshal` array of `[Fixnum, String,
  String]` and refuses the type byte `l` (Bignum: a sign byte, a length in 16-bit words, the
  words).
- Patches to the vendored interpreter are recorded in `PATCHES.md` with patch files and
  manifest hashes.

## Decisions

- Patch `NODE_CLASS` so that a superclass mismatch defines a new class under the same name in
  the same scope, exactly as the branch for an undefined constant does, without a warning
  (the original prints none). Record the patch.
- `Win32API`: lookup order is the given name, then the name with `A`. The three new functions
  go into the table; `CopyFileA` reuses the directory rule of the profile functions.
- Harness: keep an entry's identifier as its raw `Marshal` bytes when reading and write those
  bytes back.

## Risks / Trade-offs

- [Scripts that relied on the `TypeError`] → None can exist for a runtime that never raised it.
