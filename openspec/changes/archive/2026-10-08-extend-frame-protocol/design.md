# Design

## Context

See `proposal.md` for motivation. The state this change starts from:

- `vkxp-proto` is at `PROTOCOL_VERSION` 1. `FramePacket` has `frame_index`, `logical_size`,
  `bitmaps`, `ops`. `Quad` has `clip`, `bitmap`, `src`, `transform`, `opacity`, `blend`.
  `GuestToHost` is `Frame`, `Freeze`, `Transition`, `Audio`, `Modal`, `SetTitle`, `Shutdown`;
  `HostToGuest` is `Input`, `ModalDismissed`, `Tick`, `Quit`. `Button` has nineteen variants and
  lacks F9.
- `vkxp-compose-ref` and `vkxp-vk` implement `Clear`, `Quad` and `RectEffect` and must agree
  byte for byte (`specs/vulkan-compositor`). The Vulkan backend validates packets like the
  reference, encodes each op as a 23-word record (`records.rs`) and evaluates it in
  `shaders/compose.comp`; `shaders/pixel.glsl` mirrors `vkxp-pixel` function for function.
- `vkxp-pixel` already has `apply_tone` and `blend_color`, both of which keep the alpha of the
  pixel they are given. They are used today for `RectEffect`.
- The dependency check in `xtask/src/deps.rs` only forbids guest crates from reaching host
  crates.

## Goals / Non-Goals

**Goals:**

- Make every protocol change the core scene objects and the host window need, in one version
  bump.
- Keep every frame that does not use the new fields byte-identical, so the committed expected
  frames do not change.
- Keep the two backends in exact agreement on the new quad path.

**Non-Goals:**

- Bush depth, plane and tilemap ops, and the rendering of freeze and transition.
- The host's pacing behaviour. This change defines the messages; the host change specifies and
  tests what the host does with them.
- A compositor trait shared by the backends.
- New pixel arithmetic. No function is added to `vkxp-pixel`.

## Decisions

### Quad tone and colour act on the sampled texel

For a covered pixel the quad computes

```text
t = sampled texel
t = apply_tone(t, tone)          alpha of t kept
t = blend_color(t, color)        alpha of t kept
pixel = blend_<mode>(pixel, t, opacity)
```

Tone first, then colour, then opacity and blending. This is how an RGSS sprite treats its own
`tone` and `color`: they recolour the sprite, not what is behind it. It reuses the two existing
arithmetic functions, so nothing is added to `vkxp-pixel` or `pixel.glsl` and the provenance
list is unchanged (both stay textbook placeholders until the original's sprite loop is
reverse-engineered).

Both functions are identities for their neutral inputs (`Tone::NEUTRAL`; a colour of alpha 0),
which is what keeps existing frames unchanged. A test proves the identity for all 256 channel
values rather than relying on reading the formulas.

Alternative considered: emitting a `RectEffect` after the quad. Rejected because a rectangle
effect changes the pixels underneath as well, which is the viewport behaviour, not the sprite
behaviour.

### Frame rate as a range-checked value

`FramePacket` gains `frame_rate: FrameRate`. `FrameRate` wraps the number of frames per second,
can only be constructed from 10 to 120 (a clamping constructor serves `Graphics.frame_rate=`,
which clamps in the original runtime) and validates on decode, so an out-of-range value in a
byte stream is a decode error rather than something each backend has to check. The default is
40. Compositors ignore the field; it is for the host's pacing.

Alternative considered: a separate `SetFrameRate` message. Rejected because the rate would then
be state the host has to track across messages; carrying it in every packet keeps a packet
self-describing and a recording replayable from any point.

### New messages and the pacing handshake

`GuestToHost::FrameReset` and `HostToGuest::Reset` are appended to their enums. Their meaning,
and the handshake the existing `Tick` belongs to, are stated in the type documentation:

- The host answers every `Frame` with exactly one `Tick`. The guest does not send the next
  `Frame` before it has that `Tick`. A `Tick` carries no data, so guest behaviour does not
  depend on wall-clock time.
- `FrameReset` tells the host to restart its pacing schedule from now instead of catching up.
- On `Reset` the guest restarts the game at its next frame boundary; on `Quit` it shuts down.

The specs of this change only require that these messages exist and survive encoding. What the
host does on its side is specified with the host change, where it can be tested.

### F9

`Button::F9 = 29` is added and `Button::ALL` grows to twenty. The value was read from the
original runtime: the constants A, C, Y, L, SHIFT, ALT, F5, F7 and F9 are defined there with
11, 13, 15, 17, 21, 23, 25, 27 and 29, matching the existing discriminants.

### Version 2 and fixtures

`PROTOCOL_VERSION` becomes 2 and version 1 streams are rejected. No byte stream is committed:
`vkxp_compose_ref::fixtures` builds the streams in code and only the expected frames are stored.
The existing fixture builders set the neutral tone and zero-alpha colour, so their expected
frames must come out unchanged; new fixtures are added for the new path.

### Vulkan record layout

The op record already has a `color` word and four `tone_*` words that only `Clear` and
`RectEffect` use. A quad now fills them too, so `RECORD_WORDS` stays 23 and the shader's quad
case applies `apply_tone` and `blend_color` to the fetched texel before the blend switch.
Alternative considered: separate quad fields in a longer record; rejected as unnecessary
growth of a buffer that is rewritten every frame.

### Reverse dependency rule

`xtask` gains a second direction: `vkxp-vk`, `vkxp-audio`, `vkxp-host` and `vkxp-compose-ref`
must not reach any guest crate through normal or build dependencies, with all features enabled,
exactly as the existing rule is evaluated. The `vkxp` binary is exempt from both directions. A
violation reports the dependency path, like the existing rule.

## Risks / Trade-offs

- [The original applies tone and colour in another order or with other rounding] → The order
  lives in one place per backend and the arithmetic stays marked as placeholder; replacing it
  later is a change to those functions, not to the protocol.
- [An existing expected frame changes] → A task checks that no committed expected file is
  modified; the neutral-identity test explains why.
- [CPU and shader disagree on the new path] → Generated packets and a dedicated sweep over
  grey amount, tone offsets and colour alpha, compared byte for byte.
- [Hardware run touches the wrong GPU] → The task names the device and the loader restriction;
  the Intel Arc B580 on the development machine is reserved for other work and must not be
  opened.

## Migration Plan

None beyond the version bump. No persisted data uses version 1.
