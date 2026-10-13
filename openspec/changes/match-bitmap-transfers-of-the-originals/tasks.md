# Tasks

## 1. Transfers

- [x] 1.1 Opacity above 255; the clip of a mirrored transfer of equal sizes. Verify: the scenarios of "Block transfer"; `cargo test -p vkxp-pixel -p vkxp-core --locked` passes.
- [x] 1.2 A disposed source in the core and in `blt` and `stretch_blt`. Verify: the scenarios of "Disposal" and of "Drawing methods"; `cargo test -p vkxp-core -p vkxp-binding --locked` passes.

## 2. Values and decoding

- [x] 2.1 The limits of `Color`. Verify: the scenarios of "Colour components stored as in the original"; the same command.
- [x] 2.2 The alpha of a 32-bit BMP. Verify: the scenarios of "BMP"; `cargo test -p vkxp-image --locked` passes.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate match-bitmap-transfers-of-the-originals --strict` passes.
