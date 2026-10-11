# Local patches to Oniguruma 5.9.6

Every vendored file is byte-identical to the release unless an entry below says otherwise.
`MANIFEST.sha256` always keeps the upstream hash; a patched file must instead match the
`Patched SHA-256` of its entry. The manifest test of `vkxp-ruby-sys` reads this file, so every
entry keeps the four fields `File`, `Patch`, `Patched SHA-256` and `Reason` in this form, and a
file that several patches touch records the hash after the last of them in every entry.

To add a patch: change the file, store the unified diff (`diff -u`, paths relative to this
directory with `a/` and `b/` prefixes) under `patches/`, and add an entry.

## 1. class-range-start-is-class

- File: `regparse.c`
- Patch: `patches/class-range-start-is-class.patch`
- Patched SHA-256: `486aa643d65c93818d47121e0b65241fe8f24ab9a2339ffaab2ae0169f8fd255`
- Reason: found by the first run of the tests on another machine (change
  `fix-findings-of-the-first-ci-runs`), where `/[\d-z]+/` matched `"1-z"`. In
  `parse_char_class` a class such as `\d` leaves the state `CCS_VALUE` with the type
  `CCV_CLASS` and never sets `vs`; a following `-` makes the state `CCS_RANGE`, and the next
  character reaches `next_state_val`, which read the uninitialised `*vs` as the start of the
  range, so the result depended on what the compiler left on the stack. `next_state_val` now
  returns `ONIGERR_MISMATCH_CODE_LENGTH_IN_CLASS_RANGE` in the state `CCS_RANGE` when the
  value before the hyphen is a class, before `*vs` is read. That is the error of the
  original runtime, "mismatch multibyte code length in char-class range". A hyphen followed
  by the end of the class or by `&&` never reaches that state and stays literal.
