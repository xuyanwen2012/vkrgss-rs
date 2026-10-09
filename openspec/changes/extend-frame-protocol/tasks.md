# Tasks

## 1. Protocol types (`vkxp-proto`)

- [ ] 1.1 Add `FrameRate` (10 to 120, default 40, checked and clamping constructors, validation on decode) and the field `FramePacket::frame_rate`. Verify: unit tests for both bounds and the clamp; a test that a stream carrying a frame rate of 9 or 121 fails to decode; `cargo test -p vkxp-proto`.
- [ ] 1.2 Add `tone: Tone` and `color: Color` to `Quad` and document the order (tone, colour, then opacity and blend). Verify: the no-float test still passes and a unit test builds a quad with the neutral values.
- [ ] 1.3 Add `GuestToHost::FrameReset`, `HostToGuest::Reset` and `Button::F9 = 29`, and document the tick handshake and the meaning of both messages on the types. Verify: a test round-trips one message of every guest-to-host and host-to-guest kind and asserts reset and quit differ; the button tests cover twenty buttons and assert F9 is 29.
- [ ] 1.4 Set `PROTOCOL_VERSION` to 2 and update the crate documentation. Verify: a test decodes a stream with version 1 in its header and gets an error naming versions 1 and 2; the property tests in `tests/codec_props.rs` generate the new fields and pass.

## 2. Reference compositor (`vkxp-compose-ref`)

- [ ] 2.1 Apply the quad's tone and then its colour to the sampled texel before blending. Verify: one test per new scenario in `specs/frame-composition/spec.md` (neutral values change nothing for every channel value, full-alpha colour, tone before colour, pixels underneath untouched), and the existing scenario tests pass.
- [ ] 2.2 Set neutral tone and zero-alpha colour in the existing fixture builders and add fixtures for quad tone and quad colour in each blend mode, with their expected frames. Verify: `cargo test -p vkxp-compose-ref` passes twice in a row and `git status --short crates/vkxp-compose-ref/tests/expected` shows only added files, no modified ones.

## 3. Vulkan backend (`vkxp-vk`)

- [ ] 3.1 Encode a quad's tone and colour into the existing tone and colour words of the op record and apply them in `shaders/compose.comp` before the blend. Verify: the fixture comparison in `cargo test -p vkxp-vk` passes for all fixtures including the new ones, on the default device (llvmpipe).
- [ ] 3.2 Extend the generated-packet strategy with random quad tone and colour, and add the quad tone and colour sweep of `specs/vulkan-compositor/spec.md`. Verify: `cargo test -p vkxp-vk` passes on llvmpipe with the validation layer reporting nothing.
- [ ] 3.3 Run the backend's tests on hardware. Verify: `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk` passes and the commit message records device and driver. The Intel Arc B580 must not be used or opened.

## 4. Dependency rule (`xtask`)

- [ ] 4.1 Add the host-to-guest rule to the dependency check: `vkxp-vk`, `vkxp-audio`, `vkxp-host` and `vkxp-compose-ref` must not reach a guest crate; the `vkxp` binary is exempt. Verify: unit tests feed synthetic graphs (a forbidden direct edge, a forbidden transitive path, a development-only edge that is allowed, the binary depending on both sides) and assert the reported paths; `cargo xtask deps` passes on the workspace.
- [ ] 4.2 Describe the rule in the "Dependency rules" section of `README.md`. Verify: the README names the four crates and the exemption of the binary.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `cargo xtask gate` exits 0 and `openspec validate extend-frame-protocol --strict` passes.
