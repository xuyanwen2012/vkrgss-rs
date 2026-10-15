# Tasks

## 1. Hardening flags

- [ ] 1.1 The flags in `base_build()` of both build scripts, left out of sanitizer and fuzz builds as `design.md` says; record a patch in `PATCHES.md` for any source correction the check needs. Verify: scenarios "Flags of the build" (a test that reads the constants or the build script) and "The suites pass with them": `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys --locked` passes, and `cargo xtask stress` and `cargo test -p vkrgss-ruby-sys --test sanitized -- --ignored upstream_suites_pass_under_the_sanitizers` pass if clang with its sanitizer runtimes is installed (say so in design.md if it is not).

## 2. Workflows

- [ ] 2.1 Pins, `CC`, the conditional `cancel-in-progress` in `ci.yml` and `release.yml`; `nightly.yml`; the test of `xtask` that reads the workflow files. Verify: scenarios "Pinned inputs", "Pushes to main are not cancelled", "Scheduled checks", "Gate in the workflow", "Release only for a tag", and the aarch64 job is unchanged apart from pins and `CC`; `cargo test -p xtask --locked` passes.

## 3. Documentation

- [ ] 3.1 `docs/building.md` names the hardening flags and the compiler of the release; `docs/development.md` names the scheduled workflow; `CHANGELOG.md` under Unreleased. Verify: the tests of `xtask` that check the documents and their links pass.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate harden-the-c-build-and-pin-ci-inputs --strict` passes.
