# Tasks

## 1. Workflow

- [ ] 1.1 `cargo-deny` 0.20.2, `deny.toml` as far as known, the upload of artifacts, and the tests of `xtask` for the workflow. Verify: scenarios of "The nightly fuzz job keeps its findings"; `cargo test -p xtask --locked` passes.

## 2. Stack overflow in fuzz runs

- [ ] 2.1 The classification of `design.md` in both crates' fuzz tests, with unit tests of the classification on sample reports. Verify: scenarios of "Stack exhaustion by nesting is set aside in a fuzz run"; `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked` passes; `cargo xtask fuzz regex --seconds 600` ends without a finding.

## 3. Findings of the parsers

- [ ] 3.1 Run `parse192` and `parse` as `design.md` says; fix, record and replay what is found, or list it. Verify: a second run of 600 s of each ends without a finding that is not listed; `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked` passes; the sanitized runs pass.

## 4. Documentation

- [ ] 4.1 `docs/development.md`, `docs/vendored-security.md`, `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate make-the-nightly-checks-pass --strict` passes.
