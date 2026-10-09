# vkxp

A Rust + Vulkan runtime for RPG Maker XP (RGSS1) games. Linux first, MIT licensed.

The project is at its foundation stage: the workspace, the guest/host protocol, a CPU
reference compositor and a headless Vulkan compositor exist; Ruby, windowing and audio do not
yet. `docs/decisions.md`
records the settled design, and `openspec/` holds the change proposals that implement it.

## Layout

The guest side (script thread) and the host side (window, GPU, audio) exchange only plain data
defined in `vkxp-proto`.

| Crate | Responsibility | Side |
|---|---|---|
| `vkxp-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkxp-pixel` | Bit-exact pixel arithmetic with its provenance | shared |
| `vkxp-core` | Value types, bitmaps, scene objects, frame extraction | guest |
| `vkxp-image` | PNG, JPEG, BMP decoding | guest |
| `vkxp-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkxp-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkxp-ruby-sys` | Build of the embedded Ruby and raw FFI | guest |
| `vkxp-binding` | RGSS Ruby API on top of core | guest |
| `vkxp-compose-ref` | CPU reference compositor | host and tests |
| `vkxp-vk` | Vulkan backend | host |
| `vkxp-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkxp-host` | Event loop, input mapping, thread orchestration, configuration | host |
| `vkxp` | Executable | |

`xtask` holds repository automation and is not part of the runtime.

## Headless proof

The contract between the two sides is testable without a window, Ruby or a GPU. The example
below reads an encoded stream of `GuestToHost` messages, runs it through the CPU reference
compositor and writes the last frame as PNG:

```sh
cargo run -p vkxp-compose-ref --example render_frame -- <messages> <output.png>
```

`vkxp_compose_ref::fixtures` generates the test streams from procedural patterns. Their expected
frames are committed under `crates/vkxp-compose-ref/tests/expected/` and compared byte for byte;
`VKXP_BLESS=1 cargo test -p vkxp-compose-ref --test fixtures` rewrites them after an intended
change, to be reviewed before committing.

## Prerequisites

Building the workspace compiles the shaders of `vkxp-vk`, and testing it renders with Vulkan:

| Needed for | Prerequisite | Typical package |
|---|---|---|
| build | `glslc` on `PATH`, to compile GLSL to SPIR-V | `glslc` or `shaderc` |
| build | `spirv-val` on `PATH`, to validate the SPIR-V | `spirv-tools` |
| test | a Vulkan 1.3 device; the Mesa software rasteriser lavapipe (llvmpipe) is enough | `mesa-vulkan-drivers` |
| test, optional | the Khronos validation layer `VK_LAYER_KHRONOS_validation` | `vulkan-validation-layers` |

A missing build tool stops the build with a message naming it. Compiled shaders are written to
the build directory only.

The tests of `vkxp-vk` render every message stream with both the CPU reference compositor and
the Vulkan backend and require byte-identical frames. By default they run on the CPU device
(llvmpipe), so the result does not depend on the hardware in the machine, and they fail rather
than skip when no such device exists. `VKXP_TEST_DEVICE` runs the same tests on another device;
it holds a device index or a part of the device name, as `vulkaninfo --summary` prints it:

```sh
VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk
```

When the validation layer is installed the tests enable it, and any validation error fails the
test that caused it.

Choosing a device still enumerates every device the Vulkan loader finds. To keep a driver from
being loaded at all, restrict the loader itself, for example
`VK_LOADER_DRIVERS_SELECT='*lvp*'` for lavapipe only.

## Merge gate

One command checks everything a change must pass:

```sh
cargo xtask gate
```

It runs these steps in order, stops at the first failure and names the step that failed:

| Step | Command |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `test` | `cargo test --workspace` |
| `openspec` | `openspec validate --all` |

The toolchain is pinned in `rust-toolchain.toml`. The `openspec` CLI must be on `PATH`.

## Dependency rules

Three rules keep the guest/host boundary intact. A test in `xtask` checks them against
`cargo metadata` as part of the `test` step, and `cargo xtask deps` runs the check alone. A
violation is reported with the full dependency path that causes it.

1. **Guest crates do not reach host crates.** None of `vkxp-core`, `vkxp-image`, `vkxp-text`,
   `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding` may depend, directly or transitively, on
   `vkxp-vk`, `vkxp-audio` or `vkxp-host`.
2. **Host crates do not reach guest crates.** None of `vkxp-vk`, `vkxp-audio`, `vkxp-host`, and
   not the reference compositor `vkxp-compose-ref` either, may depend, directly or transitively,
   on a guest crate. The `vkxp` binary is exempt from both directions: it is the one crate that
   depends on both sides and joins them.
3. **`vkxp-core` stays pure.** Everything `vkxp-core` reaches, directly or transitively, must be
   on its allow-list: `vkxp-proto`, `vkxp-pixel` and the pure crates those need. The list is
   `CORE_ALLOW_LIST` in `xtask/src/deps.rs`; adding to it is a reviewed decision, and nothing
   that does file I/O or binds Ruby, Vulkan, windowing or audio belongs there.

The check runs with every feature of every workspace crate enabled, so an optional dependency
or an off-by-default feature cannot hide a forbidden path. Only normal and build dependencies
count; dev-dependencies are not linked into either side.

In addition, `vkxp-proto`, `vkxp-pixel`, `vkxp-core` and `vkxp-compose-ref` carry
`#![forbid(unsafe_code)]`, so `unsafe` in them does not compile.
