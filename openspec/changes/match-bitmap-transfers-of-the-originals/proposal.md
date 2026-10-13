## Why

While the bitmap operations of RGSS2 were measured on 2026-10-12, the same probes were run
on the DLL of XP (`RGSS103J.dll`) in the same prefix, and the outputs of the two originals
were compared byte for byte (`vx-v1-bitmap-REPORT.md` sections 5a, 5d and 10 under
`ref/tools/oracle/measurements/`, outside the repository). Five rules that vkxp follows
for both runtimes are not what either original does.

## What Changes

- A transfer with an opacity above 255 draws nothing (vkxp limits it to 255).
- A transfer of equal sizes with a negative source width or height is clipped before it is
  mirrored, by the four clips of the original.
- A disposed source bitmap in `blt` and `stretch_blt` raises nothing and blends the
  destination rectangle onto itself (vkxp raises `RGSSError`).
- A 32-bit BMP whose alpha bytes are all 0 decodes transparent (vkxp makes it opaque).
- `Color` limits red, green and blue to -255 to 255 on both ends (vkxp limits the upper end
  only).
- All five hold for RGSS1 and RGSS2 alike.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-bitmap`: disposal as a source, the opacity and the mirrored clip of a transfer, the
  limits of `Color`.
- `rgss-bitmap`: `blt` and `stretch_blt` with a disposed source.
- `image-decoding`: the alpha of a 32-bit BMP.

## Impact

- `crates/vkxp-pixel`, `crates/vkxp-core` (`bitmap.rs`, `color.rs`), `crates/vkxp-image`
  (BMP), `crates/vkxp-binding` (`bitmap.rs`, `color.rs`).
- The seven-game regression of XP is run after the merge; a change in its numbers is
  examined before the change is archived.
