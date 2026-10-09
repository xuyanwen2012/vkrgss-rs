# Proposal

## Why

The design record settles how the window behaves: it opens at the largest integer scale that
fits the screen, shows the frame with nearest-neighbour sampling at integer scales and sharp
bilinear otherwise, and Alt+Enter toggles borderless fullscreen. Today the frame is scaled with
one fixed sampling and there is no fullscreen. This is the scaling-filter part of milestone M5.

## What Changes

- The presenter scales with a selectable filter: nearest, linear, and sharp bilinear (nearest
  inside each source pixel with a one-screen-pixel linear edge). `auto`, the default, is
  nearest when the scale is a whole number and sharp bilinear otherwise.
- The window opens at the largest integer scale that fits the monitor's work area.
- Alt+Enter and F11 toggle borderless fullscreen on the window's monitor.
- Option `--filter auto|nearest|linear|sharp` and `--fullscreen`, with matching configuration
  keys.

Out of scope: post-process shader presets, higher internal resolution, exclusive fullscreen.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: scaling filters.
- `host-session`: initial window size and fullscreen toggle.
- `game-launch`: the two options.

## Impact

- Crates `vkxp-vk` (present shader and its parameters), `vkxp-host` (window), `vkxp` (options
  and configuration keys), README.
