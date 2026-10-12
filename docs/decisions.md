# vkxp design decisions

Record of the decisions settled on 2026-10-08 before any code was written. Treat these as
settled; change one only through an OpenSpec change that says so.

## Goal and scope

- vkxp is a from-scratch runtime for RPG Maker XP (RGSS1) games, built to a 2026 standard.
- Personal project, MIT licence. RGSS2/RGSS3 (VX, VX Ace) are out of scope.
- Linux is the primary platform. Android is deferred. macOS is a distant maybe.
- Compatibility bar: games using the default scripts or common community scripts run
  unmodified. Games that depend heavily on Windows may need patches.
- Fidelity: bitmap operations, blending and tone are pixel-exact against the original runtime.
  Text only needs matching layout and advance widths, not identical glyph rasters.
- Target behaviour is RGSS 1.04. `Library=` in `Game.ini` is ignored except that a trailing `J`
  or `E` selects the default font and error-message language.

## Architecture

### Guest / host split

- **Guest** (script thread): Ruby interpreter, core scene objects, CPU bitmap operations, file
  access, text rasterisation.
- **Host** (main thread plus audio thread): winit event loop, Vulkan, audio device, input.
- Only plain data crosses the boundary: frame packets, audio commands, input snapshots, modal
  message requests. The guest never holds a GPU, window or audio handle.
- The guest is a thread of the host's process or a child process, with which the same
  messages cross a Unix stream socket (change `run-the-guest-in-a-process`). It is chosen by
  `--guest thread` or `--guest process`, the environment variable `VKXP_GUEST` and the
  configuration key `guest`. Without a choice it is a process in a window and a thread in a
  headless run (change `save-and-load-states`).
- Save states are copies of the guest process: the guest forks at a frame boundary and the
  copy is parked, in memory, until it is loaded; to load, the running guest is ended and the
  copy resumed, which first leaves a new copy of itself (change `save-and-load-states`). F3
  saves and F4 loads, with Shift they choose one of ten slots. States end with the run; the
  music goes on where it was (change `loop-midi-decode-wma-and-resume-music`). The guest process confines itself with Landlock
  before the first script runs: it writes only below the game directory, executes nothing
  and opens no TCP connection (change `confine-the-guest-process`).
- The guest is deterministic: time comes from frame ticks and a host-provided clock, input is
  sampled once per `Input.update`, the random seed can be injected. Input record and replay is a
  first-version test facility.

### Compositor

- The guest emits a backend-agnostic, ordered op list per frame, in logical coordinates and
  fixed point: sprites, planes, tilemap layers, window parts, rectangle effects (viewport tone,
  colour, flash).
- The host bins ops on the CPU into 16x16-pixel tiles.
- One compute dispatch evaluates the ops per pixel, in order, with integer arithmetic and
  `texelFetch` from a bindless `texture2D[]`, writing an RGBA8 storage image. No blend state.
- A tilemap layer is an op type evaluated in the shader, not expanded into per-tile quads.
- Freeze and transition are a final per-pixel mix. A fragment pass scales the result to the
  swapchain.
- A pure-Rust CPU reference compositor evaluates the same op list and must match the GPU output
  bit for bit. It is a library used by the tests and tools; a switch in the executable that
  renders with it is reserved, not implemented.
- Arithmetic starts with textbook formulas and is tightened op by op as reverse-engineering
  specs of the original's inner loops land.
- Blending is done on non-linear values (UNORM formats, not SRGB formats), as the original does.

### Vulkan

- Through `ash`. Vulkan 1.3 core plus descriptor indexing is the only hard requirement.
- Optional, with fallback: `VK_EXT_present_timing` / `VK_KHR_present_wait2` for 40 fps pacing,
  `VK_EXT/KHR_swapchain_maintenance1`.
- Not used: descriptor heap, shader object, host image copy.
- `gpu-allocator` for memory. Shaders are GLSL 4.60, compiled with `glslc` at build time and
  checked with `spirv-val`.

### Crates

| Crate | Responsibility | Side |
|---|---|---|
| `vkxp-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkxp-pixel` | Bit-exact pixel arithmetic (blend, tone, colour, hue) with its reverse-engineering source | shared |
| `vkxp-core` | Value types, `Bitmap` pixels and operations, scene objects, frame extraction | guest |
| `vkxp-image` | PNG, JPEG, BMP decoding | guest |
| `vkxp-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkxp-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkxp-ruby-sys` | Build of the embedded Ruby and raw FFI | guest |
| `vkxp-binding` | RGSS Ruby API on top of core | guest |
| `vkxp-compose-ref` | CPU reference compositor | host and tests |
| `vkxp-vk` | Vulkan backend: binning, compositing, presentation, pacing | host |
| `vkxp-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkxp-host` | Event loop, input mapping, thread orchestration | host |
| `vkxp` | Executable: command line, configuration files | |

`vkxp-core` does no file I/O and depends on neither Ruby nor Vulkan.

## Ruby

- Embedded interpreter: vendored Ruby 1.8.7-p374 source under `third_party/ruby-1.8.7/`, with
  its own licence files, built by `vkxp-ruby-sys`.
- Required flags: `-std=gnu89 -fwrapv -fno-strict-aliasing -fPIC`. Without the first two the
  build succeeds but is wrong (misdetected configuration, broken Bignum overflow), so Ruby's own
  core test suite (625 tests) is part of the merge gate.
- The vendored tree is our fork. Differences from the original's Ruby 1.8.1 found by oracle probe
  scripts are patched back when a game could observe them.
- Exposed surface matches the original: core library only, plus `Zlib` (extension compiled in,
  `Zlib.crc_table` fixed for 64-bit, zlib linked statically) and a `Win32API` shim. Load path
  cleared and `$KCODE='U'` at start.
- Regex: the original uses Oniguruma, and so does vkxp: Oniguruma 5.9.6 is vendored and built
  in place of the interpreter's own engine (change `embed-oniguruma`); a probe of 65 patterns
  gives the original's results.
- The interpreter runs only on the script thread, with a large dedicated stack
  (`RUBY_INIT_STACK` at thread entry, `ruby_set_stack_size()` after init). Host signal handlers
  are restored after `ruby_init`. FFI declarations are hand-written.
- `system`, backticks, `exec`, `fork`, `syscall`, `IO.popen` and similar are stubs by default
  (log only), with a configuration switch. `require` and `load` only load `.rb` files inside the
  game directory.
- `Win32API`: the class exists; common calls are emulated from a table; unknown calls log once
  and return 0; preload scripts may override.
- vkxp-specific extensions live under a `VKXP` module only.
- The built-in Enterbrain Ruby source (`RPG::Sprite`, `RPG::Weather`, `RPG::Cache`, the `RPG::`
  data classes) is never committed. A tool extracts it from the user's own DLL into a
  git-ignored file that is loaded at run time.

## Platform and behaviour defaults

- `winit` + `cpal`, `gilrs` for gamepads, `symphonia` for decoding, an in-house mixer, a built-in
  SoundFont synthesiser for MIDI.
- JPEG: if a Rust decoder cannot match IJG 6b bit for bit, libjpeg-turbo is allowed.
- Encodings: UTF-8 first, then auto-detection among GBK, Shift-JIS and Big5, with a config
  override. Path lookup is case-insensitive and accepts backslashes.
- Saves are written to the game directory.
- Window: resizable; initial size is the largest integer scale that fits; nearest-neighbour at
  integer scales, sharp bilinear otherwise; Alt+Enter is borderless fullscreen.
- Pacing: logic and display at exactly the game's frame rate (default 40), no interpolation.
- `print`, `p` and script errors show a blocking in-window modal panel and log to stderr; a
  switch makes `p` log-only. Settings are TOML (global plus per-game override) and CLI flags.
- Fonts: fontconfig lookup plus a substitution table; fonts in `~/.config/vkxp/fonts/` take
  priority; no CJK font is bundled.
- SoundFont: configured path, then `/usr/share/soundfonts`.
- The original's 10-second hang watchdog is off by default.
- Command line: `vkxp <game directory>`.

## Feature tiers

- First version: scaling filters, accurate pacing, preload scripts, fast-forward, input record
  and replay.
- Reserved in the design, not implemented: higher internal resolution, post-process filters
  (planned route: RetroArch slang presets via `librashader`, to be verified), frame
  interpolation, widescreen through a run-time logical resolution.
- Added after the first version: save states (F3 and F4), see "Guest / host split".
- Not planned: rewind.

## Process

- Workspace: `~/Desktop/rgss/` holds `ref/` (installer, original DLLs, unpacked DLL, RTP,
  tools), `games/` (local test games), `vkxp/` (this repository), `wt/` (worktrees).
  Nothing under `ref/` or `games/` is ever committed.
- Everything in the repository is English. Rust edition 2024, toolchain pinned.
- `unsafe` only in `vkxp-vk`, `vkxp-ruby-sys`, `vkxp-binding` and platform code.
  `thiserror` in libraries, `anyhow` in the binary, `tracing` for logs.
- Merge gate: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
  `cargo doc` with warnings denied, `openspec validate`.
- OpenSpec proposals are drafted interactively and reviewed by the owner. Each change is
  implemented in its own branch and worktree by an actor/reviewer flow that the owner launches.
  At most three non-overlapping module changes run in parallel.
- Oracle: the original `Game.exe` under Wine. Committed tests use only procedurally generated
  images; screenshots containing RTP or game art stay local.
- Reverse engineering uses Ghidra headless; there is no restriction on reading the original DLL.

## Milestones

Status as of 2026-10-09. "Evidence" names what was checked; "Not covered" is what the status
does not claim.

The owner confirmed on 2026-10-09 that all milestones M0 to M6 are complete as recorded here,
accepting the limits listed under "Not covered". Work beyond them is new work.

| | Outcome | Status | Evidence | Not covered |
|---|---|---|---|---|
| M0 | Workspace skeleton, guest/host protocol, oracle harness | Done | `cargo xtask oracle` compares a game frame by frame with the original runtime under Wine; used on eight game set-ups | |
| M1 | Title screen of the test game, navigable | Done | The test game has no title screen and goes straight to its map; its menu, and the title screens of two other games, are navigable and compared with the original | |
| M2 | Map: tilemap, viewports, planes, walking | Done | Map frames of the test game, a second large game and three more games are identical to the original pixel for pixel; tilemaps are evaluated by the compositor as decided | |
| M3 | Audio: Ogg, WAV, MP3, MIDI | Done | Decoding and mixing tests; the owner confirmed sound in a windowed run of a real game | What the original does with more than one MIDI loop marker; WMA without `ffmpeg` installed |
| M4 | Full play-through: battle, menus, save/load, F12 reset | Done | Battle course, menu and a save written at the same frame (byte for byte) equal the original; reset is covered by tests of vkxp alone | A complete play-through of a long game by a person; F12 cannot be injected into the original for comparison |
| M5 | Modernisation: filters, present-timing pacing, configuration, gamepad | Done | Scaling filters, fullscreen, layered TOML configuration, gamepads, input recording and replay; `VK_EXT_present_timing` pacing runs on the AMD GPU at 60, 120 and 144 Hz with a mean interval of 25.0 ms | Whether present-timing pacing looks smoother than timer pacing on a physical display has not been judged by a person |
| M6 | Wider compatibility: encrypted archives, `Win32API` shims, more games | Done | A packed game is identical to the original on every compared frame with both runtimes reading only the archive; common `Win32API` calls and raw key state are emulated; 20 of 23 further XP games start and the three others stop on files missing from the projects | An archive written by the original packing tool; custom DLLs that games ship |

Text: positions, widths and line placement follow rules measured on the original under Wine
with an open font; glyph shapes are not compared, as the scope in "Goal and scope" says, and
the rules have not been measured with the fonts of Windows itself.

## Known risks

- The original DLL is ASProtect-protected; whether it runs under Wine is unverified.
- Pixel exactness requires reverse engineering every inner loop and will arrive incrementally.
- Ruby 1.8.7's conservative GC depends on compiler behaviour; aarch64 is untested.
- The vendored Oniguruma (5.9.6) is newer than the one inside the original; differences beyond
  the probed patterns are possible.
- Substitute fonts will not reproduce the original's advance widths.
- MIDI timbre will differ from the original's Windows synthesiser.
