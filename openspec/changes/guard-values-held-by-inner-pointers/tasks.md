# Tasks

## 1. The two patches

- [x] 1.1 Patches of `bignum.c` and `marshal.c` from the study, the manifest and `PATCHES.md`, and the tests of the scenarios. Verify: the scenarios of `specs/embedded-ruby-1-9-2/spec.md`; `cargo test -p vkrgss-ruby192-sys --locked` passes.

## 2. Audit

- [ ] 2.1 The audit of `design.md`, its list under "Audit", and the guards it adds as recorded patches. Verify: `cargo test -p vkrgss-ruby192-sys --locked` and `cargo test -p vkrgss-ruby192-sys --locked --test upstream_stress` pass; the sanitized run passes if available.

## 3. Documentation

- [ ] 3.1 `CHANGELOG.md` under Unreleased, Fixed; `docs/vendored-security.md` if it lists collector fixes. Verify: the tests of `xtask` that check the documents pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate guard-values-held-by-inner-pointers --strict` passes; `design.md` says that aarch64 is verified after the merge.
