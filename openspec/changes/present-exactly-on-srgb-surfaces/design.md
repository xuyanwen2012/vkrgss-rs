# Design

## Context

- `choose_format` in `crates/vkxp-vk/src/compositor/present.rs` prefers three `UNORM`
  formats, then three sRGB formats with a flag that makes `present.frag` apply the inverse
  transfer (`FLAG_SRGB_TARGET`), then `formats.first()`.
- The presentation pass uses dynamic rendering with one image view per swapchain image.
- `VK_KHR_swapchain_mutable_format` (with `VK_KHR_image_format_list`, core in 1.2) lets a
  swapchain be created with `MUTABLE_FORMAT` and a list of view formats.
- Headless surfaces on llvmpipe offer `UNORM` formats, so the sRGB path is not reached by the
  tests through a real surface; `choose_format` is tested with made-up format lists.

## Decisions

- The device enables `VK_KHR_swapchain_mutable_format` when it is offered and records it in
  its capabilities.
- When the chosen surface format is one of the three sRGB formats and the extension is
  enabled, the swapchain is created with `SwapchainCreateFlagsKHR::MUTABLE_FORMAT` and an
  `ImageFormatListCreateInfo` of the sRGB format and its `UNORM` twin; the image views and
  the pipeline's colour format use the twin; the shader flag is not set.
- Without the extension the existing inverse-transfer path stays.
- The last resort of `choose_format` takes the first format whose colour space is
  `SRGB_NONLINEAR`, then the first of any.
- Tests: the selection as unit tests with format lists (which path is taken with and without
  the extension, the last resort); and one test that forces the mutable-format path on a
  surface that also has a `UNORM` format, by a `#[cfg(test)]` switch that makes the presenter
  choose the sRGB format, presents frames and resizes under validation. A read-back of the
  swapchain image is not required.
- Under "Checked" at the end of this file: whether llvmpipe and the AMD GPU
  (`VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV`) offered the extension and an
  sRGB format on the headless surface, and whether the forced test ran on each.

## Risks / Trade-offs

- [The forced path cannot run where the headless surface has no sRGB format] → The test
  skips with a message there; the unit tests of the selection still run.

## Follow-ups

- Removing the inverse-transfer path once no supported driver needs it.

## Checked

Run on 2026-10-10 with Mesa 26.2.4 and the validation layer active, on a headless surface
(`VK_EXT_headless_surface`).

| | llvmpipe (`VK_LOADER_DRIVERS_SELECT='*lvp*'`) | AMD GPU, RADV (`VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV`) |
|---|---|---|
| `VK_KHR_swapchain_mutable_format` offered and enabled | yes | yes |
| sRGB format on the headless surface | yes, `B8G8R8A8_SRGB` | yes, `B8G8R8A8_SRGB` |
| Forced test `frames_are_presented_and_resized_on_a_forced_srgb_swapchain` | ran, passed: swapchain of `B8G8R8A8_SRGB` drawn through views of `B8G8R8A8_UNORM`, no validation message | ran, passed: the same formats, no validation message |

- Both surfaces also offer a `UNORM` format, so without the test switch neither reaches the
  sRGB path, and what is presented there is unchanged.
- The inverse-transfer fallback was not run on a device: both devices have the extension. Its
  selection is covered by the unit tests of `choose_format`.
- The forced test does not read the swapchain image back, as decided above.
