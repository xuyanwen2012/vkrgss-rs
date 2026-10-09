# Tasks

## 1. Patches (`third_party/ruby-1.8.7`, `vkxp-ruby-sys`)

- [x] 1.1 Patch the generator's real number and record the patch. Verify: tests for the five scenarios of the modified requirement in `specs/embedded-ruby/spec.md`; the earlier test that pinned the 53-bit float is replaced.
- [x] 1.2 Patch `String#hash` and `Float#hash` and record the patches. Verify: tests for the scenarios of the two hash requirements.
- [x] 1.3 Adjust the upstream tests that pin old values. Verify: the upstream suites pass under `cargo test -p vkxp-ruby-sys`.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-ruby-181-reals-and-hashes --strict` passes.
