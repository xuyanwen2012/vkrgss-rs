# Design

## Context

See the proposal. `move-core-tests-out-of-line` did the first step of this for `vkxp-core`.
The tests of `vkxp-vk` compare every frame of the backend with the CPU reference byte for
byte, so a move that changes behaviour shows.

## Decisions

### This is a move

Code is moved between files and visibility is widened only as far as the move needs
(`pub(super)`, `pub(crate)`). No function body is rewritten, no name of a public item
changes, no test is dropped or weakened. A defect found on the way is listed under
"Follow-ups" and left as it is.

### Tests out of line

The `mod tests` of `compositor/present.rs`, `device.rs` and `compositor.rs` each become a
file `tests.rs` beside the module (or a file under the existing `src/tests/` directory where
that is where their neighbours are), declared with `#[cfg(test)] mod tests;`.

As built: `compositor.rs` had no `mod tests` of its own (its tests already live under
`src/tests/`), so two modules moved: `device/tests.rs` and `compositor/present/tests.rs`.
The paths of the tests (`device::tests::…`, `compositor::present::tests::…`) are unchanged.

### Modules by concern

`compositor/present.rs` becomes a directory module. The split follows the sections the file
already has; the names below are the intended ones and may be adjusted where a boundary
turns out to lie elsewhere:

- `scaling`: `Mix`, `Filter`, `letterbox`, the whole-factor test and the encoding of the push
  constants;
- `format`: `PresentFormat`, the choice of the surface format, the swapchain extent;
- `swapchain`: `Swapchain`, `PresentFence`, the per-format pipelines;
- `presenter`: `Presenter`, `DrawCall`, `PassTarget`;
- the `impl VkCompositor` block, in two or more files by what the methods do (for example
  the state of the mix and the mask, showing a frame, and reading a frame back).

As built, under `compositor/present/`, with `present.rs` keeping the module documentation
and the re-exports:

- `scaling`: as above, with `Mask`, `Sources` and `Placement`;
- `format`: as above;
- `swapchain`: `Swapchain`, `PresentFence`, `FormatPipeline`, the two timeouts, and the
  methods of `VkCompositor` that make, replace and repair the swapchain
  (`ensure_swapchain`, `take_present_fence`, `release_undrawn`, `mark_swapchain_stale`,
  `uses_swapchain_maintenance`);
- `presenter`: `Presenter`, `DrawCall`, `PassTarget`, `record_pass`, and the methods that
  every draw starts with (`ensure_presenter`, `prepare_sources`, `submit_or_idle`);
- `state`: the mix, the filter, the mask, the frame rate and the window size;
- `held`: `freeze` and the held frame and the stand-in of the mask;
- `readback`: `scale_frame` and the off-screen draw;
- `show`: `present` and `submit_drawn`.

What was `pub(super)` in `present.rs` and is used by `compositor.rs` (`Presenter`, `Mask`,
`Presenter::forget_bound`, `Presenter::destroy`) is now `pub(in crate::compositor)`, which
is the same visibility one level further down; what the sibling modules and the tests use
of each other is `pub(super)`.

`compositor.rs` is split the same way, by the sections it has. `device.rs` needs no split
once its tests are out.

After the change no file under `crates/vkxp-vk/src` is longer than 1000 lines, except files
that hold only tests.

### One probe per module

Fields, counters and records that exist only for tests become fields of one probe type per
module (`present` and `compositor`), defined in a file of its own. The type exists in every
build; in a build without tests it has no fields and its methods are empty and inlined, so
the production code calls `self.probe.something()` without an attribute. Methods that only
tests call move into an `impl` block marked `#[cfg(test)]` in the probe's file or the test
file. After the change `#[cfg(test)]` stands in the non-test files of `compositor/` and in
`compositor.rs` only on `mod` declarations and whole `impl` blocks, and inside the probe
files. A hook that cannot move without changing behaviour stays, and is listed here with the
reason.

## Risks

- [A moved function captures something else by name] → the compiler and the byte-exact
  tests catch it; the Vulkan tests are also run on the AMD GPU by the owner at the merge.

## Follow-ups

None yet.
