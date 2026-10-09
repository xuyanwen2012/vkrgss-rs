# Proposal

## Why

vkxp has settled design decisions (`docs/decisions.md`) but no code. Every later module change
(core, Ruby binding, Vulkan backend, audio) needs a fixed workspace layout and a fixed data
contract between the guest and host sides, or parallel work will collide. This change lands that
foundation and proves it end to end with the smallest possible frame.

## What Changes

- Create the Cargo workspace with the thirteen crates from `docs/decisions.md`, pinned toolchain,
  lint configuration and a single gate script.
- Enforce the dependency rules mechanically: guest crates cannot depend on host crates,
  `vkxp-core` forbids `unsafe` and has no I/O, Ruby or Vulkan dependency.
- Define the guest/host protocol in `vkxp-proto`: frame packet, ordered op list, bitmap deltas,
  transition request, audio commands, input snapshot, modal message request. All plain data with
  a stable binary encoding.
- Define compositing semantics for the first op kinds (clear, quad with normal/add/subtract
  blending and opacity, rectangle effect) and implement them in `vkxp-compose-ref` with textbook
  arithmetic placed in `vkxp-pixel`.
- Add a headless path that turns a serialized frame packet into an image, so the contract is
  testable without a window, Ruby or a GPU.

Out of scope: Vulkan, Ruby, windowing, audio playback, tilemap/plane/window ops, exact
reverse-engineered arithmetic, the Wine oracle. Their crates exist only as empty stubs here.

## Capabilities

### New Capabilities

- `workspace-structure`: the crate layout, the allowed dependency directions and the merge gate
  that every later change must pass.
- `guest-host-protocol`: the plain-data messages exchanged between the guest and host sides and
  their encoding guarantees.
- `frame-composition`: how an ordered op list turns into pixels, and the requirement that every
  compositor backend produces identical output.

### Modified Capabilities

None.

## Impact

- New repository content: `Cargo.toml` workspace, `crates/*`, `rust-toolchain.toml`, `xtask` or
  script for the gate, CI-equivalent local command.
- New dependencies limited to serialization and error handling in the crates touched here.
- All later changes build on the types and rules introduced here; changing them afterwards is a
  breaking change to every backend.
