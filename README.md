# vkxp

A Rust + Vulkan runtime for RPG Maker XP (RGSS1) games. Linux first, MIT licensed.

The project can start a game: `vkxp <game directory>` runs the game's scripts in the embedded
Ruby interpreter against the RGSS classes and shows the frames of the Vulkan compositor in a
window, or writes them to files in a headless run. Audio commands are accepted and dropped,
there is no configuration file or gamepad support yet, and encrypted archives are not read.
`docs/decisions.md` records the settled design, and `openspec/` holds the change proposals
that implement it.

## Layout

The guest side (script thread) and the host side (window, GPU, audio) exchange only plain data
defined in `vkxp-proto`.

| Crate | Responsibility | Side |
|---|---|---|
| `vkxp-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkxp-pixel` | Bit-exact pixel arithmetic with its provenance | shared |
| `vkxp-core` | Value types, bitmaps, scene objects, frame extraction | guest |
| `vkxp-image` | PNG, JPEG, BMP decoding | guest |
| `vkxp-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkxp-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkxp-ruby-sys` | Build of the embedded Ruby and raw FFI | guest |
| `vkxp-binding` | RGSS Ruby API on top of core | guest |
| `vkxp-compose-ref` | CPU reference compositor | host and tests |
| `vkxp-vk` | Vulkan backend | host |
| `vkxp-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkxp-host` | Event loop, input mapping, thread orchestration, configuration | host |
| `vkxp` | Executable | |

`xtask` holds repository automation and the tool that extracts the built-in scripts; it is not
part of the runtime.

## Third-party source

`third_party/ruby-1.8.7/` holds the part of the Ruby 1.8.7-p374 release that `vkxp-ruby-sys`
builds into the embedded interpreter. The MIT licence of this repository does not cover that
directory: it stays under Ruby's own terms (the Ruby licence or the GPL version 2, with the
exceptions its `LEGAL` file lists, such as the LGPL for `regex.c`). Its `README.vkxp.md` states
the origin and how the subset was chosen, `MANIFEST.sha256` the upstream hash of every file and
`PATCHES.md` every local change.

## Headless proof

The contract between the two sides is testable without a window, Ruby or a GPU. The example
below reads an encoded stream of `GuestToHost` messages, runs it through the CPU reference
compositor and writes the last frame as PNG:

```sh
cargo run -p vkxp-compose-ref --example render_frame -- <messages> <output.png>
```

`vkxp_compose_ref::fixtures` generates the test streams from procedural patterns. Their expected
frames are committed under `crates/vkxp-compose-ref/tests/expected/` and compared byte for byte;
`VKXP_BLESS=1 cargo test -p vkxp-compose-ref --test fixtures` rewrites them after an intended
change, to be reviewed before committing.

## Running the demo

The host can also be driven without Ruby, by a guest written in Rust, `vkxp_host::demo`. It
moves a sprite with the arrow keys (faster with Shift), tints the middle of the screen while
Space, Enter or C is held, and swaps the background with a transition on Escape or X.

In a window:

```sh
cargo run -p vkxp-host --example demo -- <device>
```

`<device>` is the index of a Vulkan device or a part of its name, as `vulkaninfo --summary`
prints them. It has no default, so that the GPU that drives the window is always a choice. The
host keys are F12 (reset), Alt+Enter (borderless fullscreen), F2 (frame rate in the title) and
Tab held (fast-forward). Closing the window asks the guest to quit.

On the development machine the Intel Arc B580 is reserved for other work and must not be
opened, so the demo is run on the AMD integrated GPU with the loader restricted to its driver,
or on the CPU:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' cargo run -p vkxp-host --example demo -- RADV
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo run -p vkxp-host --example demo -- llvmpipe
```

Without a window, on a virtual clock that never waits:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' \
    cargo run -p vkxp-host --example demo-headless -- --frames 30 --dump 10=target/frame.png
```

The loader restriction is part of the command, not decoration: `--device` only chooses among
the devices the loader enumerates, and enumerating loads the driver of every GPU in the machine.
On the development machine every headless run is restricted to lavapipe like this.

| Option | Meaning |
|---|---|
| `--frames N` | Stop after N frames. Required: the demo runs until it is asked to quit. |
| `--dump N=PATH` | Write frame N, counted from 0, as a PNG file at the logical size. Repeatable. |
| `--input SCRIPT` | Buttons held from given frames on, for example `"5:RIGHT, 12:RIGHT+SHIFT, 20:"`. |
| `--device DEVICE` | Index of a Vulkan device or a part of its name. Defaults to `llvmpipe`. |

A script is a list of `frame:BUTTONS` entries separated by commas or line breaks; `BUTTONS` is
a `+`-joined list of button names (`DOWN`, `LEFT`, `RIGHT`, `UP`, `A`, `B`, `C`, `X`, `Y`, `Z`,
`L`, `R`, `SHIFT`, `CTRL`, `ALT`, `F5` to `F9`) or nothing, and each entry sets what is held
from that frame on. Two runs with the same options write byte-identical files, and a dumped
frame equals what the CPU reference compositor produces for the same messages.

## Running a game

### The built-in scripts

A game expects the `RPG` module of the original runtime: the data classes, `RPG::Cache`,
`RPG::Sprite` and `RPG::Weather`. That Ruby source is Enterbrain's and is not part of this
repository. The original runtime keeps it as plain text inside its DLL, and one command copies
it from a DLL of your own into a file that `vkxp` loads:

```sh
mkdir -p ~/.local/share/vkxp
cargo xtask extract-rpg <RGSS DLL> ~/.local/share/vkxp/rpg_builtin.rb
```

The DLLs of versions 1.00 and 1.02 (`RGSS100J.dll`, `RGSS102E.dll`, `RGSS102J.dll`) hold the
text as it is. Later ones are compressed by their protector, and the command says so and
writes nothing; an unpacked copy of such a DLL works. `vkxp` looks for the file at
`$XDG_DATA_HOME/vkxp/rpg_builtin.rb`, which is `~/.local/share/vkxp/rpg_builtin.rb` unless
`XDG_DATA_HOME` is set, or takes the file named with `--builtin-scripts`. Without the file it
ends with a message that names the command above.

### Options

```text
vkxp [GAME_DIR] [OPTIONS]
```

`GAME_DIR` is the directory that holds `Game.ini`; without it the current directory is used.
The game directory becomes the working directory of the scripts, so saves are written there.
Paths on the command line are relative to the directory the program was started in.

| Option | Meaning |
|---|---|
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`. Repeatable. A game that names a package without this option does not start. |
| `--builtin-scripts FILE` | The built-in scripts file, instead of the default location. |
| `--device INDEX\|NAME` | The Vulkan device: its index or a part of its name, as `vulkaninfo --summary` prints them. In a window the default is the first discrete GPU, then an integrated one; headless it is `llvmpipe`. |
| `--font-dir DIR` | A directory of fonts. Repeatable; earlier directories come first, then the game's `Fonts` directory, then the fonts installed on the system. |
| `--encoding ENCODING` | The text encoding of `Game.ini`: `utf-8`, `gbk`, `shift-jis` or `big5`. Without it UTF-8 is tried, then those three in that order. |
| `--debug` | Debug mode: `$DEBUG` is `true`. |
| `--allow-process-functions` | Lets `system`, backticks, `exec`, `fork`, `syscall` and `IO.popen` start processes. Without it they do nothing. |
| `--headless` | Run without a window, on a virtual clock that never waits. |
| `--frames N` | Headless: stop after N frames if the game has not ended. |
| `--input SCRIPT\|@FILE` | Headless: the buttons held from given frames on, in the syntax described under *Running the demo*, or a file that holds such a script. |
| `--dump FRAME=FILE` | Headless: write frame `FRAME`, counted from 0, as a PNG file. Repeatable. |

An unknown option or a malformed value ends the program with the usage message and status 2.
The exit status is 0 when the game ends normally: its scripts return or call `exit`, the
window is closed, or the frame limit of a headless run is reached. It is non-zero when a
script error ends the game (its report is on the error stream), when the host fails, when a
frame is rejected in a headless run, or when a frame that was to be dumped was never reached.

### In a window

```sh
cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --device <device>
```

The host keys are those of the demo: F12 (reset), Alt+Enter (borderless fullscreen), F2
(frame rate in the title) and Tab held (fast-forward). Closing the window asks the game to
quit.

Without `--device` the program chooses a GPU by itself, and whatever it chooses, the Vulkan
loader loads the driver of every GPU to enumerate them. On the development machine the Intel
Arc B580 must not be opened, so every run there restricts the loader and names the device:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --device RADV
```

### Headless

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> \
    --headless --frames 120 --input "40:C, 42:, 60:DOWN, 62:" --dump 100=target/frame.png
```

Nothing in a headless run depends on real time, so two runs with the same options write
byte-identical files. A script that loops without calling `Graphics.update` never sees the
request to quit that follows the last frame; the program waits five seconds for the game and
then ends with a message and a non-zero status.

The end-to-end test of the `vkxp` crate runs the built program this way on games it assembles
itself, with a stand-in for the built-in scripts. Like the other tests that render, it refuses
to run while the loader is not restricted, and it passes the restriction on to the program.

## Prerequisites

Building the workspace compiles the shaders of `vkxp-vk` and the embedded Ruby interpreter of
`vkxp-ruby-sys`, and testing it renders with Vulkan:

| Needed for | Prerequisite | Typical package |
|---|---|---|
| build, test | a C compiler, gcc or clang, to compile the vendored Ruby and zlib | `gcc` or `clang` |
| build | the development files of libcrypt (`crypt.h`, `libcrypt.so`) | `libxcrypt-devel` or `libcrypt-dev` |
| build | `glslc` on `PATH`, to compile GLSL to SPIR-V | `glslc` or `shaderc` |
| build | `spirv-val` on `PATH`, to validate the SPIR-V | `spirv-tools` |
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


## Merge gate

One command checks everything a change must pass. Its `test` step renders with Vulkan, so the
loader restriction described under Prerequisites is part of it:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate
```

It runs these steps in order, stops at the first failure and names the step that failed:

| Step | Command |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `test` | `cargo test --workspace` |
| `openspec` | `openspec validate --all` |

The toolchain is pinned in `rust-toolchain.toml`. The `openspec` CLI must be on `PATH`.

## Dependency rules

Three rules keep the guest/host boundary intact. A test in `xtask` checks them against
`cargo metadata` as part of the `test` step, and `cargo xtask deps` runs the check alone. A
violation is reported with the full dependency path that causes it.

1. **Guest crates do not reach host crates.** None of `vkxp-core`, `vkxp-image`, `vkxp-text`,
   `vkxp-vfs`, `vkxp-ruby-sys`, `vkxp-binding` may depend, directly or transitively, on
   `vkxp-vk`, `vkxp-audio` or `vkxp-host`.
2. **Host crates do not reach guest crates.** None of `vkxp-vk`, `vkxp-audio`, `vkxp-host`, and
   not the reference compositor `vkxp-compose-ref` either, may depend, directly or transitively,
   on a guest crate. The `vkxp` binary is exempt from both directions: it is the one crate that
   depends on both sides and joins them.
3. **`vkxp-core` stays pure.** Everything `vkxp-core` reaches, directly or transitively, must be
   on its allow-list: `vkxp-proto`, `vkxp-pixel` and the pure crates those need. The list is
   `CORE_ALLOW_LIST` in `xtask/src/deps.rs`; adding to it is a reviewed decision, and nothing
   that does file I/O or binds Ruby, Vulkan, windowing or audio belongs there.

The check runs with every feature of every workspace crate enabled, so an optional dependency
or an off-by-default feature cannot hide a forbidden path. Only normal and build dependencies
count; dev-dependencies are not linked into either side.

In addition, `vkxp-proto`, `vkxp-pixel`, `vkxp-core` and `vkxp-compose-ref` carry
`#![forbid(unsafe_code)]`, so `unsafe` in them does not compile.
