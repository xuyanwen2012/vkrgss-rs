# Tasks

## 1. Protocol and scene

- [x] 1.1 `FramePacket::brightness`, `TransitionRule` in the transition message, the protocol version; the scene keeps and extracts the brightness; the binding compiles and sends the rule of RGSS1. Verify: the scenarios of `specs/guest-host-protocol/spec.md` and `specs/core-scene/spec.md`; `cargo test -p vkxp-proto -p vkxp-core -p vkxp-binding --locked` passes.

## 2. Compositors

- [ ] 2.1 The CPU reference applies the brightness as the last step of a frame. Verify: the scenarios of `specs/frame-composition/spec.md`; `cargo test -p vkxp-compose-ref --locked` passes.
- [ ] 2.2 The Vulkan compositor does the same; the fixtures and the generated packets that compare it with the reference cover brightness values other than 255. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-vk --locked` passes, and so does `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk --locked`.

## 3. Session

- [ ] 3.1 The steps of a transition by its rule. Verify: the scenarios of `specs/host-session/spec.md`; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host --locked` passes.

## 4. Documentation

- [ ] 4.1 `docs/architecture.md`: the brightness of a packet and the rule of a transition, where the packet and the transition are described. Verify: the text names the fields as the code does.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate show-brightness-and-rgss2-transition-steps --strict` passes.
