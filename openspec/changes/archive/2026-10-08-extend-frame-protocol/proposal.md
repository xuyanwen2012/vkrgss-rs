# Proposal

## Why

The next modules (core scene objects, the Ruby binding, the host window) need things protocol
version 1 cannot express: the host does not know the frame rate it must pace to, cannot ask the
guest to reset (F12), a sprite cannot carry its own colour and tone, and the debug button F9
is missing. Every protocol change has to be made in both compositor backends, so it is cheapest
now, before core and host are written against version 1.

## What Changes

- **BREAKING** Protocol version 2:
  - `FramePacket` carries the frame rate the guest runs at (10 to 120, default 40).
  - `Quad` carries a tone and a colour that change the sampled texel before opacity and
    blending, leaving the pixels underneath alone.
  - New guest-to-host message `FrameReset` (`Graphics.frame_reset`) and host-to-guest message
    `Reset` (F12).
  - `Button::F9` (value 29), the twentieth `Input` constant of the original runtime.
- Both compositor backends implement quad tone and colour; fixtures, generated packets and
  arithmetic sweeps cover them. Frames of packets that do not use them stay byte-identical.
- The dependency check gains the reverse rule: host crates and the reference compositor must
  not reach guest crates.

Out of scope: bush depth, plane and tilemap ops, rendering of freeze and transition, a shared
compositor trait (it arrives with the host change, where backend selection is first needed) and
the host's pacing behaviour itself. `vkxp-pixel` is not changed: quad tone and colour reuse
`apply_tone` and `blend_color`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the frame packet carries a frame rate; the button set is the full
  RGSS1 set including F9; the control messages are enumerated and gain frame-reset and reset.
- `frame-composition`: the quad op applies a tone and a colour to the sampled texel.
- `vulkan-compositor`: the differential coverage required of the backend includes quad tone
  and colour.
- `workspace-structure`: host crates must not depend on guest crates.

## Impact

- Crates: `vkxp-proto`, `vkxp-compose-ref`, `vkxp-vk` (op records, `compose.comp`, tests),
  `xtask` (dependency check). `README.md` documents the new rule.
- Encoded streams of protocol version 1 are rejected by version 2 decoders. Nothing persisted
  depends on version 1: fixtures are generated in code and only their expected frames are
  committed, and those must not change.
- Later changes (`vkxp-core` scene objects, `vkxp-host`) build on version 2.
