# Proposal

## Why

The central bet of `docs/decisions.md` is that a compute shader can evaluate the ordered op list
with integer arithmetic and match the CPU reference bit for bit. Until a second backend exists
that claim is untested, and the protocol in `vkxp-proto` has only been exercised by the code it
was written alongside. Changing the op encoding or the fixed-point format is cheap now and
expensive once core and the Ruby binding emit packets, so the Vulkan backend should prove or
break the contract before anything else is built on it.

## What Changes

- Implement a headless Vulkan compositor in `vkxp-vk`: device creation without a surface, GPU
  bitmap storage driven by `BitmapDelta`, CPU tile binning, one compute dispatch per frame that
  evaluates `Clear`, `Quad` and `RectEffect` per pixel in order, and readback of the frame.
- Mirror the arithmetic of `vkxp-pixel` and `Transform::source_position` in GLSL using 32-bit
  integers only, so no optional device feature is needed.
- Compile shaders at build time and validate the SPIR-V.
- Add differential tests that render the same message streams through `vkxp-compose-ref` and
  `vkxp-vk` and require byte-identical frames and the same class of error: the committed
  fixtures, arithmetic sweeps and randomly generated packets.
- Run those tests with the Vulkan validation layer enabled and fail on any validation error.
- Write down three behaviours the reference already has but no spec states, because a second
  backend has to reproduce them: the initial frame state, the exact quad sampling position, and
  what happens to bitmap state when a packet fails.

Out of scope: window, surface, swapchain, presentation, scaling pass, frame pacing, freeze and
transition, plane and tilemap ops, frames in flight, and any performance target. No change to
`vkxp-proto`, `vkxp-pixel` or `vkxp-compose-ref` behaviour.

## Capabilities

### New Capabilities

- `vulkan-compositor`: what the Vulkan backend requires from a device, the guarantee that its
  frames and errors match the reference, how it reports device limits, and that it runs clean
  under the validation layer.

### Modified Capabilities

- `frame-composition`: adds requirements for the initial frame state, the quad sampling
  position and the effect of a failed packet. These describe what the reference compositor
  already does; no existing requirement changes.

## Impact

- Code: `crates/vkxp-vk` (library, shaders, build script, tests). `Cargo.toml` gains workspace
  dependencies. `README.md` documents the build and test prerequisites.
- Dependencies: `ash`, `gpu-allocator` and error handling in `vkxp-vk`; `vkxp-compose-ref` and
  `proptest` as dev-dependencies of `vkxp-vk`.
- Build environment: `glslc` and `spirv-val` are required to build the workspace. A Vulkan 1.3
  device is required to run the workspace tests, so the merge gate now needs at least a software
  rasteriser (Mesa lavapipe) installed.
- `vkxp-vk` remains a host crate; the dependency-direction rules are unchanged.
