# Tasks

## 1. Interpreter (`third_party/ruby-1.8.7`, `vkxp-ruby-sys`)

- [x] 1.1 Commit the table as data, patch symbol numbering with the two counters and the mark, and record the patch. Verify: tests for the four scenarios of `specs/embedded-ruby/spec.md` at the level of `vkxp-ruby-sys`; the upstream suites pass.

## 2. Binding (`vkxp-binding`)

- [x] 2.1 Call the mark before the game's scripts. Verify: a Ruby test through `run_game` that the first new local variable of the first game script has the number 16193.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-ruby-181-symbol-ids --strict` passes.
