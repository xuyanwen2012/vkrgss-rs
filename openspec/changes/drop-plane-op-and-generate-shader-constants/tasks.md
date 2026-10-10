# Tasks

## 1. The op (`vkxp-proto`, `vkxp-compose-ref`, `vkxp-vk`)

- [x] 1.1 Remove the op kind and the reserved notion, raise the protocol version, and adjust the backends and tests. Verify: the scenarios of `specs/guest-host-protocol/spec.md`; `grep -rn "Op::Plane\|OpKind::Plane\|is_reserved" crates` prints nothing; the workspace tests pass.

## 2. The constants (`vkxp-vk`)

- [x] 2.1 Add the shared constants file, generate the GLSL include, and remove the hand-written twins from the shaders. Verify: the scenarios of `specs/vulkan-compositor/spec.md`; no constant of the shared file is defined in a file under `shaders/`.

## 3. Documents

- [x] 3.1 Bring `README.md` in line (protocol version, op list). Verify: the README names version 7 and no reserved op.

## 4. Integration check

- [x] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' env --default-signal=INT cargo xtask gate` exits 0 and `openspec validate drop-plane-op-and-generate-shader-constants --strict` passes.
