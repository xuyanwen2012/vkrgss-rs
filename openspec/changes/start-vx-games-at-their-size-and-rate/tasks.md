# Tasks

## 1. Interpreter

- [x] 1.1 The program starts the interpreter with the symbol table of the run's runtime, in both places. Verify: the scenarios of `specs/rgss-runtime/spec.md`; `cargo test -p vkxp-binding --locked` and `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` pass.

## 2. Frame size and rate

- [x] 2.1 The scene of the guest by runtime; the starting size of the window; `Graphics.width` and `Graphics.height` in RGSS2 only. Verify: the scenarios of `specs/rgss-modules/spec.md` and `specs/game-launch/spec.md`; the same two test commands pass.

## 3. Documentation

- [x] 3.1 `docs/running.md`: what a game of VX gets today (size, rate, symbol numbers) in the part that says how far it runs. Verify: the text names nothing that the code does not do.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate start-vx-games-at-their-size-and-rate --strict` passes.
