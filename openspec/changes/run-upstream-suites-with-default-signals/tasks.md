# Tasks

## 1. Harness (`vkxp-ruby-sys` tests)

- [x] 1.1 Reset the dispositions in the child and add the test. Verify: the scenarios of `specs/embedded-ruby/spec.md`; `env --ignore-signal=INT cargo test -p vkxp-ruby-sys --locked` passes.

## 2. Integration check

- [ ] 2.1 Run the full gate with SIGINT ignored. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --ignore-signal=INT cargo xtask gate` exits 0 and `openspec validate run-upstream-suites-with-default-signals --strict` passes.
