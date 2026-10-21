# Design

## Context

- On the Mac: cmake 4 and python3 from Homebrew, no ninja; network to GitHub works. Homebrew
  has `vulkan-loader` and `vulkan-headers` 1.4.363.0; `libMoltenVK.dylib` has `minos 12.0`.
- `xtask/src/app.rs` assembles `target/app/vkrgss.app` and copies the loader and MoltenVK from
  `/opt/homebrew/lib`.

## Decisions

- **Source.** `git clone --depth 1 --branch vulkan-sdk-1.4.363.0` of
  `https://github.com/KhronosGroup/Vulkan-Loader` and of `Vulkan-Headers` into
  `target/macos-loader/src/`; record the two commit hashes in `xtask` and fail on a mismatch.
  A later version is a change of these constants.
- **Build.** `cmake -S <loader> -B <build> -G "Unix Makefiles" -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_OSX_DEPLOYMENT_TARGET=14.0 -DCMAKE_OSX_ARCHITECTURES=arm64
  -DVULKAN_HEADERS_INSTALL_DIR=<headers install> -DBUILD_TESTS=OFF` (headers installed first
  with their own cmake), then `cmake --build <build> --parallel`; the result is
  `libvulkan.1.dylib` (and its version-named file); its install name is set to
  `@rpath/libvulkan.1.dylib` as the app does now.
- **The check.** Read the Mach-O load commands in Rust (a small parser of the header and
  `LC_BUILD_VERSION`/`LC_VERSION_MIN_MACOSX`, for thin arm64 and fat files) or through `otool -l`
  if simpler; compare with the minimum written into `Info.plist` (14.0); list each offending file.
- **Acceptance on this Mac.** `cargo xtask app` succeeds and its check passes; the bundle's
  `vkrgss` runs a generated game headless with a frame to dump, identical to the development
  build, from a shell without `DYLD_*` (as in `package-a-macos-app`). A run on an actual macOS 14
  is not possible here; the deployment target and the check stand for it, said in the docs.

## Out of scope

A universal (x86_64) build, Developer ID signing, building MoltenVK from source.

## Follow-ups

None yet.
