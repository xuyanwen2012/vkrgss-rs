# Design

## Context

- `vkxp-vk`'s present pass (`shaders/present.frag`) mixes the held and the newest frame and
  scales the result into the letterbox rectangle; `letterbox(frame, window)` computes it.
- `vkxp-host` creates a resizable `winit` window and maps keys through a key map value.
- `vkxp` has options and layered TOML configuration with unknown keys refused.
- The design record: initial size is the largest integer scale that fits; nearest at integer
  scales, sharp bilinear otherwise; Alt+Enter is borderless fullscreen.

## Decisions

- One fragment shader with a filter selector in its push constants. The frame images are
  sampled with a linear sampler; nearest and sharp bilinear are computed by moving the sample
  position: for sharp bilinear, `uv = (floor(p) + clamp(fract(p) * scale, 0, 0.5) +
  clamp((fract(p) - 1) * scale + 0.5, 0, 0.5)) / size` with `p` the position in frame pixels,
  which is the usual sharp-bilinear construction; nearest snaps to the pixel centre.
- The mix happens before scaling, as today, so the filter cannot change a mix at scale 1.
- The size computation is a pure function tested by itself; the window code calls it with the
  monitor's work area and falls back to the frame size when the monitor is unknown.
- The toggle is handled in the window event code before the key map sees the keys; Alt+Enter
  swallows that Enter press, F11 is not a game button.
- Tests for the filters render to the off-screen target on llvmpipe.

## Risks / Trade-offs

- [Wayland does not report a work area] → Use the monitor size; a compositor may then shrink
  the window, which the resize path already handles.
