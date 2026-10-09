# Design

## Context

See `proposal.md` for motivation. What exists:

- `vkxp-proto` defines `FramePacket`, `BitmapDelta`, `Op` (`Clear`, `Quad`, `RectEffect`, and
  the reserved `Plane` and `TilemapLayer`), `Transform::source_position` and the value types.
- `vkxp-pixel` has the arithmetic as `const fn`s on `[u8; 4]`: `effective_alpha`,
  `blend_normal`, `blend_add`, `blend_sub`, `blend_color`, `apply_tone`.
- `vkxp-compose-ref` has `Compositor::render`, which validates the logical size, applies the
  deltas, checks every op (`prepare`) and then evaluates the ops per pixel starting from
  `BACKGROUND` (opaque black). Its `fixtures` module generates the committed message streams.
- `vkxp-vk` is an empty stub. `*.spv` is already git-ignored.

Constraints from `docs/decisions.md`: `ash`; Vulkan 1.3 core plus descriptor indexing is the only
hard requirement; `gpu-allocator`; GLSL 4.60 compiled by `glslc` and checked by `spirv-val`;
16x16 tiles binned on the CPU; one compute dispatch; `texelFetch` from a bindless array; integer
arithmetic; no blend state. `unsafe` is allowed in `vkxp-vk`.

The development machine has `glslc`, `spirv-val`, the Khronos validation layer, and three
Vulkan 1.4 devices: an Intel Arc B580 (ANV), an AMD integrated GPU (RADV) and llvmpipe.

## Goals / Non-Goals

**Goals:**

- A `vkxp-vk` compositor whose frames and errors equal the reference's, proven by tests that
  run in the merge gate.
- A structure the later presentation change can extend without rewriting: the frame target
  stays on the GPU and readback is a separate step.

**Non-Goals:**

- Throughput. Rendering is synchronous: submit, wait, read back. No frames in flight, no
  upload batching beyond what is natural, no pipeline cache.
- A backend trait shared with `vkxp-compose-ref`. It is introduced when `vkxp-host` needs to
  switch backends at run time, with two real implementations to shape it.
- A CLI or example binary. Tests are the proof here; the reference already has the PNG example.

## Decisions

### Public surface

`vkxp-vk` exposes a `VkCompositor` with `new(DeviceChoice)`, `handle(&GuestToHost)` and
`render(&FramePacket)`, mirroring the reference, plus `read_frame()`-style readback returning
size and RGBA8 bytes. It defines its own `VkError` with variants that correspond one to one to
`ComposeError` and `BitmapError` kinds (same indices and ids) plus `Limit`, `Device` and
`Vulkan` variants. `vkxp-compose-ref` is a dev-dependency only.

Alternative considered: depend on `vkxp-compose-ref` for `Frame` and the error types. Rejected:
the production path never returns a CPU frame, and a normal dependency would let the GPU backend
quietly reuse reference code in the path under test. The cost is a duplicated validation routine
of about a hundred lines, which the differential error tests pin to the reference.

### Bitmap table

A slot table indexed by `BitmapId::index` holding generation, size and a `VkImage` with its
view. Semantics copy `BitmapStore`: create into an occupied slot, stale generation, out-of-bounds
update and length mismatch are the same errors. Capacity is 65 536 slots; a larger index is a
`Limit` error. Bitmap width and height are limited by `maxImageDimension2D`.

`Create` clears the image to transparent black. `Update` copies through a staging buffer. Because
rendering is synchronous, `Destroy` frees the image as soon as the packet's work has completed;
there is no deferred-destruction queue yet.

### Integer formats end to end

Bitmaps and the frame target are `R8G8B8A8_UINT`. The shader reads `utexture2D` with
`texelFetch` and writes `rgba8ui` with `imageStore`, so no value passes through a float and no
UNORM rounding rule is involved. The frame target is created with `MUTABLE_FORMAT` so the later
presentation pass can view it as `R8G8B8A8_UNORM` (same compatibility class) for scaling. This
keeps the decision that blending happens on non-linear values and nothing is sRGB.

Alternative considered: UNORM images and `uint(v * 255.0 + 0.5)` in the shader. Rejected: it
is very likely exact, but "very likely" is the wrong standard for a bit-identity guarantee.

### Descriptors

One descriptor set, update-after-bind, partially bound:

- binding 0: `utexture2D bitmaps[]`, 65 536 entries, indexed by slot index with `nonuniformEXT`;
- binding 1: frame target storage image;
- binding 2: op records (storage buffer);
- binding 3: tile headers, one `(offset, count)` per tile (storage buffer);
- binding 4: tile op indices (storage buffer).

Frame constants (logical size, tile grid width) go in push constants. Required features, checked
at device selection: `runtimeDescriptorArray`, `shaderSampledImageArrayNonUniformIndexing`,
`descriptorBindingPartiallyBound`, `descriptorBindingSampledImageUpdateAfterBind`, and an
update-after-bind sampled-image limit of at least the table capacity. All are part of the
descriptor indexing feature set that Vulkan 1.2 promoted.

### Op records

Each op becomes a fixed-size record of 32-bit words: kind, clip as `x0, y0, x1, y1` already
intersected with the screen, bitmap slot, source rectangle, the six transform coefficients as raw
fixed-point bits, opacity, blend mode, colour, and tone. Validation happens on the CPU before any
record is written, so the shader never sees an invalid slot or source rectangle. Ops whose clip
does not intersect the screen are validated but emit no record.

### Tile binning

`tiles_x = ceil(width / 16)`, `tiles_y = ceil(height / 16)`. For each record, in op order, its
index is appended to every tile its clipped rectangle intersects. The clip rectangle is used as
the bound for all three kinds; tightening quads to their transformed footprint is a later
optimisation. The binning function is pure CPU code with no Vulkan types, so it is unit-tested
without a device.

The dispatch is `tiles_x` by `tiles_y` workgroups of 16x16 invocations. Each invocation discards
itself if it lies outside the logical size, otherwise starts from opaque black, walks its tile's
index list in order and stores the result.

### Exact transform arithmetic without 64-bit shader integers

`Transform::source_position` forms `xx*X + xy*Y` exactly, shifts right by 16 with floor, then
adds the translation. With logical dimensions capped at 16 384, `X` and `Y` fit in 31 bits, each
product fits in 62 bits and the sum in 63 bits, so signed 64-bit arithmetic is sufficient. The
shader implements it with pairs of 32-bit words using `imulExtended`, `uaddCarry` and explicit
shifts. After the shift and the translation the texel column and row fit in a signed 32-bit
integer, so the comparison against the source rectangle is ordinary.

Alternative considered: `int64_t` in GLSL. Rejected: it needs the optional `shaderInt64`
feature, which would add a hard device requirement that `docs/decisions.md` rules out.

The GLSL helpers are tested directly: a test-only compute shader evaluates the transform for a
buffer of coefficient and pixel inputs, including the extremes, and the results are compared
with `Transform::source_position`.

### Pixel arithmetic in GLSL

One GLSL include file mirrors `vkxp-pixel` function for function, with the same names and the
same integer expressions, and each function carries a comment pointing at its Rust counterpart.
Drift is caught by the arithmetic-sweep test rather than by generating one from the other.

Alternative considered: generating GLSL from the Rust source, or sharing through a
Rust-to-SPIR-V compiler. Rejected as far more machinery than six small functions justify.

### Shader build

`build.rs` compiles every shader under `crates/vkxp-vk/shaders/` with
`glslc --target-env=vulkan1.3 -Werror`, runs `spirv-val --target-env vulkan1.3` on the output in
`OUT_DIR`, and the library embeds it with `include_bytes!`. A missing tool is a build error
naming the tool and the package that provides it. `cargo:rerun-if-changed` covers each source and
include.

### Device selection and tests

`DeviceChoice` is `Auto`, `Index(n)` or `Name(substring)`. Tests read `VKXP_TEST_DEVICE`; when it
is unset they select the CPU device (llvmpipe), so the gate is reproducible and does not depend
on, or disturb, whatever the hardware GPUs are doing. Setting it runs the same tests on a
hardware device. If no suitable device exists the tests fail; they never skip, because a skipped
differential test would let a broken backend through the gate.

Tests enable `VK_LAYER_KHRONOS_validation` when it is installed and install a debug-utils
messenger that records messages of error severity; a shared test harness asserts that the record
is empty at the end of every test and after the compositor is dropped.

### Differential tests

A helper renders a message stream through both backends and compares each step: both produce a
frame and the bytes are equal, or both fail with the same kind and index. It is used by:

- every stream in `vkxp_compose_ref::fixtures::all()`;
- arithmetic sweeps: 256x256 frames where the destination varies along one axis and the source
  along the other, for each blend mode at several opacities and alphas, for colour blend and for
  tone, including a transparent destination;
- `proptest` strategies for valid packet sequences (bitmaps created, updated, destroyed and
  reused across packets; ops of all kinds with arbitrary clips and transforms; logical sizes
  that are not multiples of 16) and for sequences with injected invalid deltas and ops;
- targeted cases: several thousand ops on one pixel, an op entirely off screen, the limit
  errors, the transform extremes.

Property tests use a fixed case count and persist failing seeds in the repository as `proptest`
does by default.

## Risks / Trade-offs

- [Driver bug makes one device differ] → The claim is tested per device, not assumed. A
  difference on a hardware device is recorded with the smallest failing packet and investigated;
  it does not get papered over with a tolerance.
- [Tests need a Vulkan device, so the gate no longer runs anywhere] → Lavapipe is a package
  install and is named in the README; failing loudly is preferred over skipping.
- [65 536 bitmap slots or the op-record layout turn out wrong for later ops] → Both are private
  to `vkxp-vk` and can change without touching the protocol.
- [Hand-mirrored GLSL drifts from `vkxp-pixel` when formulas are tightened later] → The sweep
  test covers every channel pair, so a one-sided change fails the gate.
- [Synchronous render and per-frame buffer allocation are slow] → Accepted here; the
  presentation change owns frame pacing and resource reuse.
- [`build.rs` needing `glslc` makes `cargo build` fail on a fresh machine] → Decided in
  `docs/decisions.md`; the error message says what to install.

## Open Questions

- Whether the frame target should later become a UNORM image written through a UINT view, or
  stay UINT with a UNORM view. Either works with `MUTABLE_FORMAT`; the presentation change picks.
