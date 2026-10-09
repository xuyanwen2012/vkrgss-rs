# Tasks

## 1. Link (`vkxp-proto`)

- [x] 1.1 Add the `link` module: `link(wake)` returning a guest end (`send`, blocking `recv`, `try_recv`) and a host end (`try_recv`, `send`), with a disconnection error. Verify: tests for the three link scenarios in `specs/guest-host-protocol/spec.md` (order, wake callback, dropped peer), run with a second thread.

## 2. Session logic (`vkxp-host`)

- [ ] 2.1 Add the dependencies and define the `Clock` and `Display` traits, a fake clock and a recording display for tests. Verify: `cargo build -p vkxp-host` has no warnings and `cargo xtask deps` reports no violation.
- [ ] 2.2 Implement pacing in `Session`: one tick per frame, the due time rule, frame-rate changes, fast-forward and frame reset. Verify: tests with the fake clock for every scenario of the frame-pacing and fast-forward-and-frame-reset requirements and of the tick handshake.
- [ ] 2.3 Implement the key map with its defaults, snapshots on change only, and release on focus loss. Verify: tests for every scenario of the input-delivery and default-key-map requirements.
- [ ] 2.4 Implement reset, quit, shutdown and disconnection handling. Verify: tests for the three scenarios of that requirement plus a dropped guest end.
- [ ] 2.5 Implement freeze, transition, title, modal and rejected-frame handling against the `Display` trait. Verify: tests using the recording display for the freeze-and-transition, title-and-modal and rejected-frames scenarios.

## 3. Presentation (`vkxp-vk`)

- [ ] 3.1 Add a compositor constructor for a window surface (surface extensions, swapchain extension, a device that can present) without changing the headless constructor. Verify: existing `cargo test -p vkxp-vk` passes unchanged; a test creates the compositor for a headless surface on llvmpipe; a test for the device-cannot-present error where it can be provoked.
- [ ] 3.2 Implement the scaling pipeline (letterbox, nearest at whole-number factors, sharp bilinear otherwise, held-frame mix) with an off-screen target and readback. Verify: tests for the scaling, sampling and transition-mix scenarios in `specs/vulkan-presentation/spec.md`, comparing read-back pixels.
- [ ] 3.3 Implement the swapchain: creation, acquire and present, recreation on resize and out-of-date, zero-sized windows, teardown. Verify: a test on llvmpipe with a headless surface presents frames, resizes, presents again and drops the backend, with the validation layer reporting nothing.
- [ ] 3.4 Implement freeze (copy to the held image) and the mix weight in the presenter, and expose them through a type that implements the host's `Display` operations. Verify: off-screen tests that a held frame is shown after later renders and that weights 0, one half and 1 give the expected images.

## 4. Runners and demo (`vkxp-host`)

- [ ] 4.1 Implement the demo guest. Verify: a test runs it against a scripted host end for 50 frames and checks it sends one frame per tick and shuts down on quit.
- [ ] 4.2 Implement `run_headless` with the virtual clock, the input script parser, frame dumps and the frame limit. Verify: tests for the four headless-run scenarios, including the comparison of a dumped frame with `vkxp-compose-ref` rendering the same messages, and parser tests for the script syntax and its errors.
- [ ] 4.3 Implement `run_windowed` on `winit` with the Vulkan display, key translation, Alt+Enter, F2, F12, Tab and close handling. Verify: it compiles on the default target and a test drives the same event-handling code with synthetic key events through the session (no window is created in tests).
- [ ] 4.4 Add the `demo` example (windowed, taking a device name or index) and a `demo-headless` example (frame dump), and document both in `README.md` including which device to pass on the development machine. Verify: `cargo run -p vkxp-host --example demo-headless -- --frames 30 --dump 10=target/frame.png` writes a PNG; the README commands match the examples' arguments.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `cargo xtask gate` exits 0, `cargo xtask deps` reports no violation, and `openspec validate add-host-window --strict` passes.
