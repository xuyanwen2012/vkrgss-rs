# Proposal

## Why

The Vulkan compositor renders frames that nobody sees: there is no window, no pacing, no input
and nothing that answers the guest's messages. The host side has to exist before any guest,
Ruby or otherwise, can be run for real, and it can be built and tested now with a guest written
in Rust.

## What Changes

- `vkxp-proto`: a link (two endpoints over channels) that carries the protocol messages between
  the guest thread and the host, with a hook that wakes the host.
- `vkxp-vk`: presentation. The compositor can be created for a window surface, owns a swapchain,
  and draws the composited frame scaled into the window with letterboxing, including the
  cross-fade of a transition. A frame can also be scaled into an off-screen target for tests and
  frame dumps.
- `vkxp-host`:
  - the session logic that consumes guest messages and produces host messages: one tick per
    frame at the packet's frame rate, fast-forward, frame reset, freeze and transition, title,
    modal, shutdown;
  - keyboard mapping to the RGSS1 buttons, reset on F12, quit on window close;
  - a windowed runner on `winit` and a headless runner with a virtual clock, scripted input and
    frame dumps to PNG;
  - a demo guest in Rust used by tests and as an example.

Out of scope: gamepads, audio, configuration files, the CPU reference compositor as a run-time
backend, mask-driven transitions (a mask is accepted and shown as a plain cross-fade),
present-timing extensions, Ruby.

## Capabilities

### New Capabilities

- `host-session`: what the host does with guest messages and user input: pacing, input
  delivery, reset and quit, freeze and transition, and the windowed and headless ways of running.
- `vulkan-presentation`: showing composited frames in a window: surface and swapchain handling
  and the scaling of the logical screen into the window.

### Modified Capabilities

- `guest-host-protocol`: adds the link between the two sides and the tick handshake as
  requirements.

## Impact

- Crates: `vkxp-proto` (new module), `vkxp-vk` (new presentation module; the headless
  compositor keeps working without a surface), `vkxp-host`. `README.md` gains a section on
  running the demo.
- New dependencies: `winit`, `ash-window`, `raw-window-handle`; `png` in `vkxp-host` for frame
  dumps.
- On the development machine the window must be driven by the AMD integrated GPU or llvmpipe;
  automated tests use llvmpipe with a headless surface and never open a window.
