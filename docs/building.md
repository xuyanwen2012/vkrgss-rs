# Building

vkrgss builds with Cargo; the toolchain is pinned in `rust-toolchain.toml`.
`cargo build --release -p vkrgss` produces the optimised executable, `target/release/vkrgss`.

## Prerequisites

Building the workspace compiles the shaders of `vkrgss-vk` and the embedded Ruby interpreter of
`vkrgss-ruby-sys`, and testing it renders with Vulkan:

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
| `cargo xtask dist` only | the static library of libcrypt (`libcrypt.a`), which the release package links, and `readelf` | `libxcrypt-static` or `libcrypt-dev`; `binutils` |

A missing build tool stops the build with a message naming it. Compiled shaders are written to
the build directory only.

The interpreter is built from `third_party/ruby-1.8.7/` by the build script of `vkrgss-ruby-sys`
with a fixed file list and fixed flags. No installed Ruby, `configure`, `make` or system zlib is
involved, and the only supported target is `x86_64-unknown-linux-gnu`. The build script also
accepts `aarch64-unknown-linux-gnu`, which is not supported: it is built with the
configuration header of x86_64, so that the CI job `gate-aarch64` can show what works there
(see [development.md](development.md#merge-gate)). Any other target stops the build with a
message naming it. `CC` selects the compiler. Because a wrong flag gives an interpreter
that builds and then computes wrong results, the tests of `vkrgss-ruby-sys` compile a Ruby
executable from the same objects and run Ruby's own test suites against it, which is why the
tests need the C compiler too.

A second interpreter is built beside it: Ruby 1.9.2-p0, the release that the original runtime
of RPG Maker VX Ace embeds, from `third_party/ruby-1.9.2/` by the build script of
`vkrgss-ruby192-sys`, in the same way: a fixed file list, the flags `-std=gnu89 -fwrapv
-fno-strict-aliasing`, a checked-in `ruby/config.h`, the zlib of `libz-sys`, and nothing
generated at build time. It is linked with the encodings the original runtime has
(UTF-16LE, Shift_JIS with Windows-31J, ISO-8859-1) and no other. Its tests compile a Ruby
executable too and run the release's `bootstraptest` and a selection of its `test/ruby`,
once plainly and once with `GC.stress = true`; `crates/vkrgss-ruby192-sys/tests/upstream.allow`
names what is left out, with the reasons. `vkrgss-binding-rgss3` compiles the source of
`vkrgss-binding` over it and runs the tests of the binding a second time
([development.md](development.md)). No executable links this interpreter yet, and no
package may link both: they define the same symbols, which a dependency rule checks.

Both build scripts give the compiler the hardening flags themselves, whatever it turns on
by default, for gcc and clang alike: `-U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=3`,
`-fstack-protector-strong` and `-fstack-clash-protection`, and on x86_64
`-fcf-protection=full`. A compiler that rejects one fails the build. `CFLAGS` of the
environment come after them, so a build of one's own can still override one. With `_FORTIFY_SOURCE` the C library
ends the process with "buffer overflow detected" where a call states a size larger than its
buffer; a place it stops at is corrected in the vendored source by a recorded patch, never
by switching the check off. The instrumented builds of the tests, the sanitized run and the
fuzz targets ([development.md](development.md#checks-outside-the-gate)), leave
`_FORTIFY_SOURCE` out, because it conflicts with the interceptors of AddressSanitizer.

The release package and the CI runs are built with gcc 13, the default compiler of Ubuntu
24.04: the workflows set `CC=gcc-13`, so that a new default of the runner image does not
change the compiler of the release unnoticed. The interpreter's collector scans the machine
stack, so the compiler shapes what it sees; changing that compiler is a change of its own.

`vkrgss-ruby192-sys` is for the target `x86_64-unknown-linux-gnu` alone. For any other target,
`aarch64-unknown-linux-gnu` included, its build script compiles nothing and prints a warning
that names the target, and the crate and its tests are empty; so is `vkrgss-binding-rgss3`,
the package that compiles the RGSS classes over it, whose build script sets the `cfg`
`vkrgss_interpreter_absent` for such a target. The rest of the workspace
builds as before. Unlike `vkrgss-ruby-sys` it is not built on aarch64 with the header of
x86_64: an interpreter with native threads and copied Fiber stacks is not offered on a
guess.

The tests of `vkrgss-vk` render every message stream with both the CPU reference compositor and
the Vulkan backend and require byte-identical frames. By default they run on the CPU device
(llvmpipe), so the result does not depend on the hardware in the machine, and they fail rather
than skip when no such device exists.

Choosing a device is not enough to leave the other GPUs alone: the Vulkan loader loads every
installed driver to enumerate the devices. The tests of `vkrgss-vk` and `vkrgss-host` therefore
refuse to start, before anything is asked of the loader, unless it has been restricted to
chosen drivers with `VK_LOADER_DRIVERS_SELECT` (or `VK_DRIVER_FILES` / `VK_ICD_FILENAMES`).
For the default device that is lavapipe only:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test --workspace
```

`VKRGSS_TEST_DEVICE` runs the same tests on another device; it holds a device index or a part of
the device name, as `vulkaninfo --summary` prints it. The loader is then restricted to that
device's driver, for example the AMD integrated GPU:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' VKRGSS_TEST_DEVICE=RADV cargo test -p vkrgss-vk
```

When the validation layer is installed the tests enable it, and any validation error fails the
test that caused it.

Presentation is tested the same way and never opens a window: scaling and mixing are drawn into
an off-screen target and read back, and the swapchain runs on a surface without a window system
(`VK_EXT_headless_surface`). The headless tests of `vkrgss-host` follow `VKRGSS_TEST_DEVICE` too
and default to llvmpipe.
