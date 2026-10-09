# Design

## Context

See `proposal.md` for motivation. What this change starts from (after `extend-frame-protocol`):

- Protocol version 2: `FramePacket` carries `frame_rate`; `GuestToHost` has `Frame`, `Freeze`,
  `Transition { duration, mask, vague }`, `Audio`, `Modal`, `SetTitle`, `FrameReset`,
  `Shutdown`; `HostToGuest` has `Input`, `ModalDismissed`, `Tick`, `Reset`, `Quit`.
- `vkxp-vk` has `VkCompositor::new(DeviceChoice)`, `render(&FramePacket)` into an
  `R8G8B8A8_UINT` frame image that stays on the device, and `read_frame()`. It creates its
  instance and device without any window-system extension.
- `vkxp-host` and the `vkxp` binary are stubs.
- Development machine: the monitor is driven by the Intel Arc B580, which must not be used by
  this project. The AMD integrated GPU (RADV) and llvmpipe can both present to a Wayland window.
  The Vulkan loader offers `VK_EXT_headless_surface`, and llvmpipe supports swapchains.

## Goals / Non-Goals

**Goals:**

- A host that a guest can be run against today: frames appear in a window at the right rate,
  keys reach the guest, reset and quit work.
- The session logic testable with a fake clock and a fake backend, without a window or GPU.
- A headless run that produces deterministic frame dumps, as the acceptance tool for every
  later change.

**Non-Goals:**

- Gamepads, audio output, a configuration file, present-timing based pacing, the CPU reference
  compositor as a run-time backend.
- Mask-driven transitions and bit-exact transitions. The cross-fade is done while presenting,
  in floating point; moving transitions into the compositors is a later change.
- Launching Ruby. The guest in this change is a Rust function.

## Decisions

### The link lives in `vkxp-proto`

Guest crates cannot depend on host crates and the other way round, so the endpoint types both
sides hold go into the shared crate, in a `link` module: `link(wake) -> (GuestEnd, HostEnd)` over
two standard channels. `GuestEnd` has `send`, a blocking `recv` and `try_recv`; `HostEnd` has
`try_recv` and `send`; `wake` is a callback the guest end invokes after each send, which the
windowed runner uses to wake the event loop. Dropping an end disconnects the other.

Alternative considered: a trait for the transport with the channel implementation in the host.
Rejected for now: one in-process implementation exists, and the later process split will replace
the module wholesale.

### Session logic separated from the platform

`vkxp-host` is three layers:

1. `Session`: a state machine with no window, thread or GPU in it. Inputs: guest messages, key
   events, focus loss, close requests, the current time. Outputs: host messages, a next-wake
   time, and requests to a `Display` trait (render a packet, hold the current frame, set the mix
   weight, set the title). It owns pacing, the key map, freeze and transition state.
2. Runners: `run_windowed` (winit `ApplicationHandler`, real clock, the Vulkan display) and
   `run_headless` (virtual clock, scripted input, frame dumps).
3. `Display` implementations: the Vulkan presenter for a window; an off-screen one for headless
   runs; a recording fake for tests.

Every pacing, input and transition scenario of `specs/host-session` is tested at layer 1 with a
fake clock and the recording display.

### Pacing

With `p` the period of the frame being answered and `t_prev` the time of the previous tick, the
tick is due at `max(t_prev + p, t_arrival)`; fast-forward makes it due at `t_arrival`; after a
frame reset it is due at `t_arrival + p`. One tick answers one frame, so there is never a
backlog to drain. The windowed runner sleeps with `ControlFlow::WaitUntil` and is woken early by
the link's wake callback through an `EventLoopProxy`.

Frames are presented as they arrive, with FIFO present mode. Pacing by presentation timestamps
(`VK_EXT_present_timing`) is a later refinement behind the same `Session` interface.

### Freeze and transition while presenting

The presenter keeps a second image, the held frame. `Freeze` copies the current frame into it
and makes it the one shown. `Transition { duration }` starts a counter; each following frame
advances it and the fragment shader shows `mix(held, newest, step / duration)`. This is
presentation-level and uses floating point, which is acceptable because frame dumps are taken
from the compositor's frame, not from the window. A mask is ignored apart from being logged
once.

### Presentation in `vkxp-vk`

`VkCompositor` gains a second constructor that takes window and display handles
(`raw-window-handle`), enables the surface extensions (`ash-window`) and `VK_KHR_swapchain`, and
only accepts a device with a queue that can present to the surface. A `Presenter` owned by the
compositor holds the swapchain and a graphics pipeline that draws one full-screen triangle
sampling the frame image (and the held image) with `texelFetch`:

- letterbox rectangle computed on the CPU and passed as constants; outside it the shader writes
  black;
- at whole-number factors the nearest texel; otherwise "sharp bilinear": the sample position is
  snapped so that only the half-texel bands at pixel borders interpolate, computed from four
  fetched texels;
- a `UNORM` swapchain format is preferred so values pass through unchanged; if the surface only
  offers sRGB formats the shader applies the inverse transfer function so the displayed values
  are the same.

The same pipeline can draw into an off-screen image of a requested size and read it back; tests
and the scaling scenarios use that path. The swapchain path itself is exercised on llvmpipe with
a headless surface.

Alternative considered: `vkCmdBlitImage` for scaling. Rejected because it cannot letterbox and
mix in one step and its filtering of integer formats is not available.

### Keys

A table maps `winit` physical keys to buttons, with the defaults of `specs/host-session`. Host
keys are separate from it: F12 (reset), Alt+Enter (borderless fullscreen), F2 (frames per second
in the title), Tab held (fast-forward). The headless runner feeds button states directly from
its script, bypassing the key map.

### Headless runner and input script

`run_headless(options, link)` renders with the headless compositor on the device chosen by the
caller (llvmpipe by default), ticks on a virtual clock, and writes requested frames with
`read_frame` and the `png` crate. The input script is text: entries `frame:BUTTONS` separated
by commas or newlines, where `BUTTONS` is a `+`-joined list of button names or empty, each entry
setting the held set from that frame on (for example `60:C, 62:, 120:DOWN+SHIFT, 124:`). The
snapshot for frame `n` is sent before that frame's tick.

### Demo guest

`vkxp_host::demo` is a guest written against `GuestEnd`: a few generated bitmaps, quads that
move with the directions, a rectangle effect on a button, freeze and transition on another,
shutdown on quit. It is the guest of the headless tests and of the `demo` example.

## Risks / Trade-offs

- [No window can be opened where the flow runs] → Nothing in the tasks needs one: the swapchain
  is tested on a headless surface and everything else off-screen. A windowed smoke test is done
  by hand on the AMD GPU.
- [llvmpipe lacks headless-surface presentation in some environment] → The test reports the
  missing capability as a failure with a clear message rather than skipping silently; the
  off-screen tests still cover scaling and mixing.
- [Float scaling differs between devices] → Only whole-number factors and flat interiors are
  asserted exactly.
- [The Intel Arc B580 gets opened by default device selection] → Tests name llvmpipe; the
  windowed runner takes the device from its caller and the README says which to pass here.

## Open Questions

- The key used for fast-forward. Tab is a placeholder; it changes a constant.
