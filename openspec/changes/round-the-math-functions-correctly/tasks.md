# Tasks

## 1. Vendored CORE-MATH

- [x] 1.1 `third_party/core-math/` with the selection of `design.md`, `LICENSE`, `README.vkrgss.md`, `FILES`, `MANIFEST.sha256` and its test. Verify: the scenarios of "Vendored source matches upstream" as the other trees have them pass for this tree.

## 2. Ruby 1.8.7

- [x] 2.1 Build CORE-MATH and the shim in `vkrgss-ruby-sys`; `-ffp-contract=off` in its flags; the recorded patch that routes the calls; the tests of the vectors and of special arguments. Verify: the scenarios of `specs/embedded-ruby/spec.md`; `cargo test -p vkrgss-ruby-sys --locked` passes, the upstream suites included.

## 3. Ruby 1.9.2

- [ ] 3.1 The same in `vkrgss-ruby192-sys`, with `lgamma_r`. Verify: the scenarios of `specs/embedded-ruby-1-9-2/spec.md`; `cargo test -p vkrgss-ruby192-sys --locked` passes, the upstream suites included.

## 4. Documentation

- [ ] 4.1 `docs/building.md` and `docs/vendored-security.md` name the new tree; the licence notices list CORE-MATH's MIT licence wherever third-party licences are listed (the release package included); `CHANGELOG.md` under Unreleased. Verify: the tests of `xtask` that check the documents, their links and the package contents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, the collector stress (`cargo xtask stress`) passes, and `openspec validate round-the-math-functions-correctly --strict` passes.
