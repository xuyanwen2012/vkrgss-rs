# Tasks

## 1. Ruby 1.9.2

- [x] 1.1 The patch of `backward_search_range` in `third_party/ruby-1.9.2`, its entry in `PATCHES.md`, and the test. Verify: the scenarios of `specs/embedded-ruby-1-9-2/spec.md`; `cargo test -p vkrgss-ruby192-sys --locked` passes; the sanitized run of `vkrgss-ruby192-sys` passes if clang with its sanitizer runtimes is installed.

## 2. Ruby 1.8.7 and Oniguruma 5.9.6

- [x] 2.1 The same in `third_party/oniguruma-5.9.6`. Verify: the scenarios of `specs/embedded-ruby/spec.md`; `cargo test -p vkrgss-ruby-sys --locked` passes; the sanitized run of `vkrgss-ruby-sys` passes if available.

## 3. Documentation

- [x] 3.1 The rows named in `design.md` of `docs/vendored-security.md`, and `CHANGELOG.md` under Unreleased, Security. Verify: the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate check-the-least-length-in-backward-searches --strict` passes.
