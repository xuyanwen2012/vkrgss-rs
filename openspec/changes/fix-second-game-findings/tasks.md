# Tasks

## 1. Interpreter (`third_party/ruby-1.8.7`, `vkxp-ruby-sys`)

- [x] 1.1 Patch class redefinition and record the patch. Verify: tests for the four scenarios of `specs/embedded-ruby/spec.md`; the upstream suites pass, with any upstream test that pins the `TypeError` adjusted and listed.

## 2. Win32API (`vkxp-binding`)

- [x] 2.1 Add the name fallback and the three functions. Verify: Ruby tests for the three scenarios of `specs/rgss-runtime/spec.md`.

## 3. Harness (`xtask`)

- [x] 3.1 Preserve Bignum identifiers in the archive reader and writer. Verify: the scenario of `specs/oracle-harness/spec.md`.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-second-game-findings --strict` passes.
