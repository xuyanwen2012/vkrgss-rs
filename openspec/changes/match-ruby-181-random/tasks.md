# Tasks

## 1. Patch (`third_party/ruby-1.8.7`, `vkxp-ruby-sys`)

- [ ] 1.1 Patch `random.c` and record the patch (patch file, `PATCHES.md`, manifest hash). Verify: tests for the four scenarios of `specs/embedded-ruby/spec.md`.
- [ ] 1.2 Adjust the upstream tests that pin the old sequences. Verify: the upstream suites pass under `cargo test -p vkxp-ruby-sys`.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-ruby-181-random --strict` passes.
