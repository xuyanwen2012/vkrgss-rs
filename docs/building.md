# Building

vkrgss builds with Cargo; the toolchain is pinned in `rust-toolchain.toml`.
`cargo build --release -p vkrgss -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3` produces the
optimised executables: the program, `target/release/vkrgss`, and beside it the guest
executables it starts, `vkrgss-guest-rgss12` for games of XP and VX and `vkrgss-guest-rgss3`
for games of VX Ace ([running.md](running.md#guest-process)).

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
accepts `aarch64-unknown-linux-gnu`, with configuration headers of its own: those that the
release's `configure` wrote on aarch64, which differ from those of x86_64 only in the
platform strings, as `tests/config_headers.rs` checks. It is built and tested by the CI job
`gate-aarch64` (see [development.md](development.md#merge-gate)) and is not supported
before the milestone P1 is accepted. It also accepts `aarch64-linux-android`
([Android](#android)) and `aarch64-apple-darwin`, with headers that `configure` wrote on a
Mac, reviewed (see [macOS](#macos)). Any other target stops the build with a message naming
it. `CC` selects the compiler. Because a wrong flag gives an interpreter
that builds and then computes wrong results, the tests of `vkrgss-ruby-sys` compile a Ruby
executable from the same objects and run Ruby's own test suites against it, which is why the
tests need the C compiler too.

A second interpreter is built beside it: Ruby 1.9.2-p0, the release that the original runtime
of RPG Maker VX Ace embeds, from `third_party/ruby-1.9.2/` by the build script of
`vkrgss-ruby192-sys`, in the same way: a fixed file list, the flags `-std=gnu89 -fwrapv
-fno-strict-aliasing -ffp-contract=off`, a checked-in `ruby/config.h`, the zlib of `libz-sys`, and nothing
generated at build time. It is linked with the encodings the original runtime has
(UTF-16LE, Shift_JIS with Windows-31J, ISO-8859-1) and no other. Its tests compile a Ruby
executable too and run the release's `bootstraptest` and a selection of its `test/ruby`,
once plainly and once with `GC.stress = true`; `crates/vkrgss-ruby192-sys/tests/upstream.allow`
names what is left out, with the reasons. `vkrgss-binding-rgss3` compiles the source of
`vkrgss-binding` over it and runs the tests of the binding a second time
([development.md](development.md)). Of the executables only `vkrgss-guest-rgss3` links this
interpreter, and no package may link both: they define the same symbols, which a dependency rule checks.

Both interpreters take the functions of `Math` and the power of floats from CORE-MATH,
which is correctly rounded, so that a float that a script computes, and a save that holds
it, is the same with every C library. Each build script compiles the files that
`third_party/core-math/FILES` lists, the functions of `third_party/core-math/src/binary64`
and the functions in `third_party/core-math/vkrgss/` through which the interpreter calls
them, into a library of its own (`-std=gnu11 -ffp-contract=off -fno-fast-math`); a recorded
patch of each interpreter makes `math.c`, `numeric.c` and `bignum.c` call them. The
interpreters and CORE-MATH are compiled with `-ffp-contract=off`, so that no compiler fuses
a multiplication and an addition on one architecture and not on another. The tests `math.rs`
of both crates check the results against committed test vectors
(`crates/vkrgss-ruby-sys/tests/data/core-math/`).

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

`vkrgss-ruby192-sys` is for the targets `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `aarch64-linux-android` ([Android](#android)) and
`aarch64-apple-darwin`, which `crates/vkrgss-ruby192-sys/targets.rs` lists. The header of
aarch64 is the one that the release's `configure` wrote there, kept with the same two
changes as that of x86_64, from which it differs only in `RUBY_PLATFORM`; that of macOS is
the one `configure` wrote on a Mac, with the same two changes. For any other
target its build script compiles nothing and prints a warning that names the target, and
sets the `cfg` `vkrgss_interpreter_absent`, under which the crate and its tests are empty;
so are `vkrgss-binding-rgss3`, the package that compiles the RGSS classes over it, and
`vkrgss-guest-main-rgss3`, whose build scripts read the same list and set the same `cfg`,
and the executable `vkrgss-guest-rgss3` only says that it is not built for the target. The
rest of the workspace builds as before.

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

## Android

Both interpreters, and with them the guest executables `vkrgss-guest-rgss12` and
`vkrgss-guest-rgss3`, build for `aarch64-linux-android` at Android API level 34 (Android 14)
or later, with the NDK (r30 is the one tested), and so does the app `vkrgss-android`
(below). The target is not supported before the milestone P3 is accepted. The Rust target
and the compiler of the NDK for API level 34 are given to cargo:

```sh
rustup target add aarch64-linux-android
TC=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$TC/aarch64-linux-android34-clang
export CC_aarch64_linux_android=$TC/aarch64-linux-android34-clang
export AR_aarch64_linux_android=$TC/llvm-ar
cargo build -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3 --target aarch64-linux-android
```

The build scripts read `__ANDROID_API__` of that compiler and stop with a message naming API
level 34 below it: bionic has `__freadahead`, through which the patched `io.c` of Ruby 1.8.7
reads the buffer of a stream, from that level on, and `backtrace`, which Ruby 1.9.2 calls,
from 33. The configuration headers of Android are those of aarch64 Linux without what bionic
lacks (`crypt`, `eaccess`, `lchmod`, `group_member`, the internals of glibc's `FILE`) and
with the platform `aarch64-linux-android`; the tests `config_headers.rs` allow no other
difference. On Android `libz-sys` links the system's `libz.so` and exports no include path:
the compiler of the NDK takes `zlib.h` from its sysroot. Neither libcrypt, libpthread nor
librt is linked; bionic has no `crypt`, so `String#crypt` raises `NotImplementedError`
there. bionic's libm never sets `errno`, and glibc before 2.43 does not report every
underflow, so the functions of `third_party/core-math/vkrgss/` set it themselves, on every
platform, as glibc 2.43 does, for `Math` of Ruby 1.8.7, which reads it.

The tests of the two interpreter crates run on a phone over adb:

```sh
ANDROID_NDK_HOME=/path/to/android-ndk-r30 cargo xtask android-test vkrgss-ruby-sys --device <SERIAL>
ANDROID_NDK_HOME=/path/to/android-ndk-r30 cargo xtask android-test vkrgss-ruby192-sys --device <SERIAL>
```

Without `--device` adb takes `ANDROID_SERIAL` or the only phone it sees. The command builds
the test binaries for the target, links Ruby's own test executable on the host from the
archives of the build script as the tests do elsewhere, pushes both with the directory of
the crate and the vendored trees its tests read into `/data/local/tmp/vkrgss-test/`, runs
every binary there with `--test-threads=1` under `umask 022`, and removes that directory
again. The output of each binary is kept in `target/android-test/<SERIAL>/<CRATE>/logs/`. On
the phone the tests take the directories that cargo and the build script named on the host
(`CARGO_MANIFEST_DIR`, `CARGO_TARGET_TMPDIR`, `VKRGSS_RUBY_SOURCE_DIR`,
`VKRGSS_RUBY192_SOURCE_DIR`) from environment variables of the same names, and the
executable from `VKRGSS_RUBY_EXECUTABLE` or `VKRGSS_RUBY192_EXECUTABLE`. The tests that
compile C at run time are ignored on Android with that reason, and the instrumented ones are
opt-in everywhere. In `/data/local/tmp` SELinux refuses hard links to the shell, so the
suites leave out `test_file_exhaustive.rb` on Android, Ruby 1.8.7 by the list `LEFT_OUT` of
its `tests/upstream.rs` and Ruby 1.9.2 by `tests/upstream.android.allow`, which also names
`String#crypt` and the report of Ruby 1.9.2 on SIGSEGV, each with its reason.

### The app

The package `vkrgss-android` is the native library of an APK whose two activities, the
library and a game, are subclasses of Android's `NativeActivity`, through winit's Android
backend; one small Java class (`crates/vkrgss-android/java/`) adds the file picker and the
start of a game. It is built and packed without Gradle:

```sh
ANDROID_NDK_HOME=/path/to/android-ndk-r30 ANDROID_HOME=/path/to/Android/Sdk cargo xtask apk
```

The command builds the library for `aarch64-linux-android` with the compiler of the NDK at
API level 34, as above, in the development profile, compiles the Java class with the
compiler of the JDK (`java -m jdk.compiler/com.sun.tools.javac.Main`, at the class version
of Java 8 against `android.jar`) and turns it into `classes.dex` with `d8`, and assembles
`target/apk/vkrgss-debug.apk` with the newest build tools of the SDK: `aapt2` links the
manifest (package `io.github.xuyanwen2012.vkrgss`, minimum API level 34, target 37,
landscape on either side, fullscreen, Vulkan 1.3 required) against
`platforms/android-34/android.jar`, the library and the two guest executables
(`libvkrgss_guest_rgss12.so`, `libvkrgss_guest_rgss3.so`) go into `lib/arm64-v8a/` without
their debug information, compressed and extracted at installation (see
[running.md](running.md)), `zipalign` aligns it to pages
of 16 KiB, and `apksigner` signs the APK with a debug key that `keytool` of the JDK makes
in `target/apk/debug.keystore` on first use. The SDK needs the build tools (35 or later,
for 16 KiB pages) and the platform `android-34`. On the desktop the package builds as a
library whose Android parts are left out, so the gate builds and tests its other parts.
How to run a game with it is in [running.md](running.md#android).

Both APKs carry the workspace's version as `versionName` and the version code
`MAJOR * 1000000 + MINOR * 1000 + PATCH` (3001 for 0.3.1), so that each release installs over
the one before.

The debug key differs from machine to machine, and Android refuses to install an APK over one
signed with another key: the app would have to be uninstalled, and its data with it. Copy
`target/apk/debug.keystore` from the machine that built the installed app to keep updating it.

### The release APK

```sh
export VKRGSS_ANDROID_KEYSTORE=/path/to/release.keystore
export VKRGSS_ANDROID_KEYSTORE_PASSWORD=... VKRGSS_ANDROID_KEY_ALIAS=vkrgss
export VKRGSS_ANDROID_KEY_PASSWORD=...
cargo xtask apk --release
```

builds the same APK in the release profile, without `debuggable`, and signs it with the key
of that keystore, with signatures of the schemes v2 and v3, into
`target/apk/vkrgss-<version>-android-arm64.apk`, with its SHA-256 checksum beside it as
`sha256sum` writes one (`sha256sum -c` checks it). The passwords reach `apksigner` as
`env:` references, so that they never appear in a command line. Without the four variables
the command refuses before it builds and names them. Every release must be signed with the
same key: Android installs an update only over an APK of the same signer, and Obtainium
does the same.

The owner makes the keystore once, keeps it and its passwords safe outside the repository
(a lost key means that the installed apps cannot be updated, only replaced), and never
commits it:

```sh
keytool -genkeypair -keystore release.keystore -storetype PKCS12 -alias vkrgss \
    -keyalg RSA -keysize 4096 -validity 10000 -dname "CN=vkrgss"
```

A PKCS12 keystore has one password for the store and its key: `keytool` ignores a
different `-keypass`, so `VKRGSS_ANDROID_KEY_PASSWORD` is the store's password too.

The release workflow (`.github/workflows/release.yml`) builds the release APK for a version
tag when the repository has these secrets, and attaches it with its checksum to the GitHub
release; without `VKRGSS_ANDROID_KEYSTORE_BASE64` it leaves the APK out with a notice:

| Secret | Value |
|---|---|
| `VKRGSS_ANDROID_KEYSTORE_BASE64` | the keystore file in base64, `base64 -w0 release.keystore` |
| `VKRGSS_ANDROID_KEYSTORE_PASSWORD` | its password |
| `VKRGSS_ANDROID_KEY_ALIAS` | the alias of the key, `vkrgss` above |
| `VKRGSS_ANDROID_KEY_PASSWORD` | the password of the key |

They are set under Settings, Secrets and variables, Actions of the repository, or with
`gh secret set VKRGSS_ANDROID_KEYSTORE_BASE64 < <(base64 -w0 release.keystore)` and
`gh secret set NAME` for the others. The workflow writes the keystore to a temporary file
of the runner and removes it after the build; it installs the NDK r30
(`ndk;30.0.16248370`), the build tools 36.0.0 and the platform `android-34` with
`sdkmanager`, and the Rust target `aarch64-linux-android`.

### The validation APK

```sh
VKRGSS_ANDROID_VALIDATION_LAYER=/path/to/android-binaries-1.4.363.0/arm64-v8a/libVkLayer_khronos_validation.so \
    cargo xtask android-run --validation <SERIAL> <GAME_DIR>
```

builds `target/apk/vkrgss-validation.apk`: the debug APK with the Khronos validation layer for
Android that the variable names (the `android-binaries-<version>.zip` of a release of
`KhronosGroup/Vulkan-ValidationLayers`, Apache-2.0, never committed) in `lib/arm64-v8a/`,
where the loader of Android finds the layers of a debuggable app. How a game runs under it is
in [running.md](running.md#validation-on-a-phone).

## macOS

The workspace builds for `aarch64-apple-darwin`, a Mac with Apple silicon, and its tests pass
there on MoltenVK; macOS is not supported yet (milestone P2 of
[decisions.md](decisions.md#macos)), and no CI job runs on it. The Mac needs Apple's command
line tools, for clang, and from Homebrew the Vulkan loader, MoltenVK, `glslc` and
`spirv-val` (`vulkan-loader`, `molten-vk`, `shaderc`, `spirv-tools`); neither libcrypt, ALSA
nor libudev, whose parts the system has. Ash does not find Homebrew's `libvulkan.dylib` by
itself, the loader is restricted to MoltenVK as lavapipe is on Linux, and the tests are told
the name of the Apple GPU:

```sh
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib
export VK_LOADER_DRIVERS_SELECT='*MoltenVK*' VKRGSS_TEST_DEVICE='Apple M4'
cargo test --workspace --locked
```

`DYLD_FALLBACK_LIBRARY_PATH` does not pass through a program of the system such as
`/usr/bin/nohup` or `/usr/bin/env`, which macOS protects: cargo is to be started from the
shell. When ash does not find the loader, the program and the tests take Homebrew's,
`/opt/homebrew/lib/libvulkan.1.dylib`, by its path. The validation layer is not installed
there, so the tests run without it. A headless run composites on the first device whose
name contains `Apple` unless `--device` says otherwise, since the Mac has no CPU
rasteriser.

On the Mac the instance asks for `VK_KHR_portability_enumeration`, without which the loader
does not enumerate MoltenVK, and a device that lists `VK_KHR_portability_subset` is created
with it. A guest process is not confined (there is no Landlock) and ends with its parent
through a kqueue. `cargo xtask dist` makes the package of Linux only and says so on macOS;
the Mac has the app instead (below).

The tests kept to Linux each say why: those that read ELF files, `/proc` or inotify, the
RGSS3 test that writes a file name that is not UTF-8, which the file system of macOS
refuses, the archive tests of `cargo xtask dist`, which use GNU tar, the test of the
metrics of a replaced font (macOS has Tahoma), and one test of each interpreter's own
suites whose output differs on macOS. The tests that make files whose names differ only in
case see that the file system of the Mac folds case, and check what such a file system
does.

### The app bundle

On the Mac, `cargo xtask app` builds the three executables for release, as `cargo xtask
dist` does on Linux, and assembles `target/app/vkrgss.app`, which runs on a Mac without
Homebrew's loader or MoltenVK and without `DYLD_*` variables, and beside it a zip of the app
written by `ditto`, `target/app/vkrgss-<VERSION>-aarch64-apple-darwin.zip`:

- `Contents/MacOS/`: `vkrgss` and the two guest executables, which it starts from beside
  itself.
- `Contents/Frameworks/`: Homebrew's Vulkan loader, `libvulkan.1.dylib`, and
  `libMoltenVK.dylib`, copied from `/opt/homebrew/lib` with the install names
  `@rpath/libvulkan.1.dylib` and `@rpath/libMoltenVK.dylib`. The executables link only
  libraries of the system, so they get no rpath.
- `Contents/Resources/vulkan/icd.d/MoltenVK_icd.json`: Homebrew's manifest of MoltenVK with
  the `library_path` `../../../Frameworks/libMoltenVK.dylib`, relative to the manifest.
- `Contents/Resources/`: the documents and licence texts of the package of Linux, and under
  `licenses/` those of MoltenVK and the loader (see [THIRD-PARTY.md](../THIRD-PARTY.md)).
- `Contents/Info.plist`: the identifier `io.github.xuyanwen2012.vkrgss`, the version of the
  workspace and macOS 14 as the minimum; no icon.

The bundle is signed ad hoc (`codesign --force --sign - --deep`), which the changed install
names need on Apple silicon; a Developer ID and notarization come with the release
([decisions.md](decisions.md#macos)). The command fails when `otool -L` of an executable or
library of the bundle names a path under `/opt/homebrew`. Homebrew's `vulkan-loader` and
`molten-vk` are needed to build the app, not to run it; the loader is built for the macOS
that Homebrew built it on, so the app runs on that macOS and later, whatever `Info.plist`
says, until the release brings a loader built for macOS 14.

A program that lies in an app bundle, `<NAME>.app/Contents/MacOS/`, loads the loader of the
bundle by its path and, unless `VK_DRIVER_FILES` or `VK_ICD_FILENAMES` is set, sets
`VK_DRIVER_FILES` to the bundle's manifest of MoltenVK, so that the loader takes that driver
and no other. How to run the app is in [running.md](running.md#macos).
