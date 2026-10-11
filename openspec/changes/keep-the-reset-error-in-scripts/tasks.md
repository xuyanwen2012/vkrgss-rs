# Tasks

## 1. The current error after a reset

- [x] 1.1 `protect` puts back the error it found; the loop over the scripts sets the `Reset` after a reset, as in `design.md`. Verify: the four scenarios of `specs/rgss-runtime/spec.md` as tests; `cargo test -p vkxp-ruby-sys -p vkxp-binding --locked` passes.

## 2. Documentation

- [x] 2.1 README: where it describes the reset, one sentence on `$!` and `$@` and the idiom it serves. Verify: the README names `unless $@`.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate keep-the-reset-error-in-scripts --strict` passes.
