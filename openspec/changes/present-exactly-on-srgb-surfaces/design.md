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
