# Building

vkxp builds with Cargo; the toolchain is pinned in `rust-toolchain.toml`.
`cargo build --release -p vkxp` produces the optimised executable, `target/release/vkxp`.

## Prerequisites

Building the workspace compiles the shaders of `vkxp-vk` and the embedded Ruby interpreter of
`vkxp-ruby-sys`, and testing it renders with Vulkan:

| Needed for | Prerequisite | Typical package |
|---|---|---|
| build, test | a C compiler, gcc or clang, to compile the vendored Ruby and zlib | `gcc` or `clang` |
| build | the development files of libcrypt (`crypt.h`, `libcrypt.so`) | `libxcrypt-devel` or `libcrypt-dev` |
| build | `glslc` on `PATH`, to compile GLSL to SPIR-V | `glslc` or `shaderc` |
| build | `spirv-val` on `PATH`, to validate the SPIR-V | `spirv-tools` |
| build | the development files of ALSA (`alsa.pc`, `libasound.so`), for the audio output | `alsa-lib-devel` or `libasound2-dev` |
| build | the development files of libudev (`libudev.pc`, `libudev.so`), for gamepads | `systemd-devel` or `libudev-dev` |
| test | a Vulkan 1.3 device; the Mesa software rasteriser lavapipe (llvmpipe) is enough | `mesa-vulkan-drivers` |
| test, optional | the Khronos validation layer `VK_LAYER_KHRONOS_validation` | `vulkan-validation-layers` |

A missing build tool stops the build with a message naming it. Compiled shaders are written to
the build directory only.

The interpreter is built from `third_party/ruby-1.8.7/` by the build script of `vkxp-ruby-sys`
with a fixed file list and fixed flags. No installed Ruby, `configure`, `make` or system zlib is
involved, and the only supported target is `x86_64-unknown-linux-gnu`; any other stops the build
with a message naming it. `CC` selects the compiler. Because a wrong flag gives an interpreter
that builds and then computes wrong results, the tests of `vkxp-ruby-sys` compile a Ruby
executable from the same objects and run Ruby's own test suites against it, which is why the
tests need the C compiler too.

The tests of `vkxp-vk` render every message stream with both the CPU reference compositor and
the Vulkan backend and require byte-identical frames. By default they run on the CPU device
(llvmpipe), so the result does not depend on the hardware in the machine, and they fail rather
than skip when no such device exists.

Choosing a device is not enough to leave the other GPUs alone: the Vulkan loader loads every
installed driver to enumerate the devices. The tests of `vkxp-vk` and `vkxp-host` therefore
refuse to start, before anything is asked of the loader, unless it has been restricted to
chosen drivers with `VK_LOADER_DRIVERS_SELECT` (or `VK_DRIVER_FILES` / `VK_ICD_FILENAMES`).
For the default device that is lavapipe only:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test --workspace
```

`VKXP_TEST_DEVICE` runs the same tests on another device; it holds a device index or a part of
the device name, as `vulkaninfo --summary` prints it. The loader is then restricted to that
device's driver, for example the AMD integrated GPU:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV cargo test -p vkxp-vk
```

When the validation layer is installed the tests enable it, and any validation error fails the
test that caused it.

Presentation is tested the same way and never opens a window: scaling and mixing are drawn into
an off-screen target and read back, and the swapchain runs on a surface without a window system
(`VK_EXT_headless_surface`). The headless tests of `vkxp-host` follow `VKXP_TEST_DEVICE` too
and default to llvmpipe.
