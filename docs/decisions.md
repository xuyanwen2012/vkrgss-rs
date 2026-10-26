# vkrgss design decisions

Record of the decisions settled on 2026-10-08 before any code was written. Treat these as
settled; change one only through an OpenSpec change that says so.

The change `extend-scope-to-vx-and-vx-ace` changed the scope on 2026-10-12, by decisions of
the owner taken that day: from RPG Maker XP alone to RGSS1, RGSS2 and RGSS3. It rewrote "Goal
and scope" and added "Several runtimes" and the rows V0 to V3 of "Milestones".

The change `plan-vx-ace` recorded the decisions of the owner of 2026-10-13 on VX Ace, taken
after V3: it rewrote "VX Ace" and "Process" under "Several runtimes" and added the rows A0 to
A3 of "Milestones" and the risks of VX Ace.

The change `plan-the-platforms-and-modernization` recorded the decisions of the owner of
2026-10-15, taken on the questions of the modernization audit of 2026-10-14: it added
"Platforms" and "Order of work", the rows P1 to P4 of "Milestones" and their risks, and the
decisions on floats in saves, Vulkan, display, audio, Linux packages and CI in the sections
where they belong.

The change `build-the-binding-over-ruby-1-9-2` added the crates `vkrgss-ruby192-sys` and
`vkrgss-binding-rgss3` to the table of crates and to the crates that may hold `unsafe`, and
under "VX Ace" what the decision on the binding became in code.

The change `draw-text-and-bitmaps-as-rgss3` added "Text and bitmaps of RGSS3" under "VX Ace"
on 2026-10-13: which DLL version vkrgss follows, the largest bitmap, and what stands for the
answers of GDI.

The change `run-rgss3-games-in-a-guest-of-their-own` added the crates `vkrgss-guest-main-rgss3`
and `vkrgss-guest-rgss3` to the table of crates, and "The guest executable and the start of
RGSS3" under "VX Ace" on 2026-10-13.

The change `measure-text-as-gdi-does-in-rgss3` replaced, on 2026-10-14, the items on the
advance, the overhangs and synthesised styles of "What stands for the answers of GDI" under
"Text and bitmaps of RGSS3", and added the items on the hinting engine, the average width,
the glyph images and why RGSS1 and RGSS2 keep their advances.

The change `release-0-3-0` set the rows A1 to A3 of "Milestones" to "Done" with their record
on 2026-10-14, and ended the experimental state of VX Ace under "VX Ace".

The change `rename-to-vkrgss` renamed the project on 2026-10-14, by a decision of the owner
taken that day: "xp" named RPG Maker XP alone, "rgss" names the three runtimes. The program,
the guest executable, the crates and the environment variables are named `vkrgss`,
`vkrgss-guest-rgss3`, `vkrgss-*` and `VKRGSS_*`, and the repository
`https://github.com/xuyanwen2012/vkrgss-rs`; versions 0.1.0 to 0.3.0 were released as vkxp.
The change rewrote the former name in this document and added the item on the name under
"Goal and scope".

The change `start-xp-and-vx-guests-from-their-own-executable` did item 4 of "Order of work"
on 2026-10-16 and added the crate `vkrgss-guest-rgss12` to the table of crates: the guest
process of a game of RGSS1 or RGSS2 is that executable, which `vkrgss` starts from beside
itself as it starts `vkrgss-guest-rgss3`, and the program no longer starts itself as a
guest. The release archive holds the three executables.

The change `build-and-test-on-macos` added to "macOS" under "Platforms", on 2026-10-17, that
a guest process on macOS is not confined, and how the workspace is built and tested there.

## Goal and scope

- vkrgss is a from-scratch runtime for the games of RPG Maker XP, VX and VX Ace, built to a
  2026 standard.
- The project is named vkrgss, for the RGSS of its three runtimes: the program `vkrgss`,
  the crates `vkrgss` and `vkrgss-*`, the repository `vkrgss-rs` (crates and executables carry
  no `-rs`, by the convention of Rust). Up to version 0.3.0 it was named vkxp; for one
  version the program reads the files and the four environment variables a player sets
  under that name, with a warning, and recordings under that name replay for good. The
  former name stays only where `xtask/tests/former-name.allow` says why.
- In scope: RGSS1, RGSS2, RGSS3. Out of scope: RPG Maker MV and MZ (JavaScript on a browser
  engine) and 2000 and 2003 (not Ruby; EasyRPG Player exists).
- VX and VX Ace are held to the standard of XP: compared frames are pixel-identical to the
  original runtime except for text glyphs, a save written at the same frame is identical
  byte for byte, and runs are deterministic.
- Rules of VX are measured on `RGSS202E.dll`; a game of the corpus is compared with the DLL
  it ships. A difference between minor versions is recorded and gets no branch of its own;
  where one shows in a comparison, V3 is reached only after the owner has accepted that
  difference explicitly. Recording it is not a passing comparison.
- Text glyphs are excluded as for XP: layout and advance widths are compared, glyph pixels
  are not. Where an operation of VX spreads glyph pixels (a snapshot of the screen, a
  blur), the comparison says how the affected area is left out.
- Personal project, MIT licence.
- Linux on x86_64 and on aarch64 (P1, accepted on 2026-10-16) is supported now. Planned
  after it, in this order: macOS (Apple silicon), Android (aarch64), Windows (x86_64); see
  "Platforms".
- Floats in saves are identical byte for byte across machines and platforms: both
  interpreters take their mathematical functions from a vendored, correctly rounded library
  (see "Ruby").
- Compatibility bar: games using the default scripts or common community scripts run
  unmodified. Games that depend heavily on Windows may need patches.
- Fidelity: bitmap operations, blending and tone are pixel-exact against the original runtime.
  Text only needs matching layout and advance widths, not identical glyph rasters.
- Target behaviour of XP is RGSS 1.04. `Library=` in `Game.ini` is planned to tell the runtime
  too (see "The runtime of a game" under "Several runtimes"); today it is ignored except that
  a trailing `J` or `E` selects the default font and error-message language.

## Architecture

### Guest / host split

- **Guest** (script thread): Ruby interpreter, core scene objects, CPU bitmap operations, file
  access, text rasterisation.
- **Host** (main thread plus audio thread): winit event loop, Vulkan, audio device, input.
- Only plain data crosses the boundary: frame packets, audio commands, input snapshots, modal
  message requests. The guest never holds a GPU, window or audio handle.
- The guest is a thread of the host's process or a child process, with which the same
  messages cross a Unix stream socket (change `run-the-guest-in-a-process`). It is chosen by
  `--guest thread` or `--guest process`, the environment variable `VKRGSS_GUEST` and the
  configuration key `guest`. Without a choice it is a process in a window and a thread in a
  headless run (change `save-and-load-states`).
- Save states are copies of the guest process: the guest forks at a frame boundary and the
  copy is parked, in memory, until it is loaded; to load, the running guest is ended and the
  copy resumed, which first leaves a new copy of itself (change `save-and-load-states`). F3
  saves and F4 loads, with Shift they choose one of ten slots. States end with the run; the
  music goes on where it was (change `loop-midi-decode-wma-and-resume-music`). The guest process confines itself with Landlock
  before the first script runs: it writes only below the game directory, executes nothing
  and opens no TCP connection (change `confine-the-guest-process`). Then it installs a
  seccomp-BPF filter, a deny-list that answers `EPERM` to the system calls no game needs:
  starting programs, tracing other processes, sockets of every family but `AF_UNIX`, mounts,
  namespaces, kernel modules and keys, `bpf`, `io_uring` (change
  `filter-the-system-calls-of-the-guest`), and the calls that signal another process,
  `kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and `pidfd_send_signal`; `tgkill`
  stays for `raise` and `pthread_kill`. Where the kernel has Landlock ABI 6 the rules also
  scope signals, so that no other process is reached by `tgkill` either; below it, and on
  Android, only code of the guest's own could (change `verify-guests-and-deny-signals`).
  The confinement per platform: on Linux Landlock and the filter; on Android the filter and
  the app's private directory, because the kernels there have no Landlock (both test phones
  answer `ENOSYS`); on macOS none yet. `--sandbox off` and `--allow-process-functions` leave
  out both. The program takes the number and user of every process that attaches from the
  kernel, refuses one whose `Attach` states another number, requires the first to be its
  child, and signals only numbers taken so; it alone makes the answers about states.
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
- Vulkan 1.3 stays the floor; requiring 1.4 is rejected. Features of 1.4 (push descriptors,
  maintenance5 and 6) are used where the device has them, once `ash` has their bindings or
  a feature is needed; no hand-written bindings for 1.4 before that.
- `gpu-allocator` for memory. Shaders are GLSL 4.60, compiled with `glslc` at build time and
  checked with `spirv-val`.

### Crates

| Crate | Responsibility | Side |
|---|---|---|
| `vkrgss-proto` | Plain data shared by both sides: frame packet, op list, audio commands, input snapshot | shared |
| `vkrgss-pixel` | Bit-exact pixel arithmetic (blend, tone, colour, hue) with its reverse-engineering source | shared |
| `vkrgss-core` | Value types, `Bitmap` pixels and operations, scene objects, frame extraction | guest |
| `vkrgss-image` | PNG, JPEG, BMP decoding | guest |
| `vkrgss-text` | Font lookup, substitution table, glyph rasterisation | guest |
| `vkrgss-vfs` | Path resolution, RTP lookup, encrypted archive | guest |
| `vkrgss-ruby-sys` | Build of the embedded Ruby 1.8.7 and raw FFI | guest |
| `vkrgss-ruby192-sys` | Build of the embedded Ruby 1.9.2 and raw FFI, under the names of `vkrgss-ruby-sys` | guest |
| `vkrgss-binding` | RGSS Ruby API on top of core, over Ruby 1.8.7 | guest |
| `vkrgss-binding-rgss3` | The source of `vkrgss-binding` compiled over Ruby 1.9.2 | guest |
| `vkrgss-guest` | The guest role: the hidden option, the guest record and the settings in it, the subscriber of the diagnostics, the attachment, the copies that are states, the sandbox and the signal settings of a guest process; links no interpreter | guest |
| `vkrgss-guest-main` | The main of a guest: what a guest is set up with, the text backend over the font engine, the guest on a thread, and the function that makes an executable a guest process | guest |
| `vkrgss-guest-main-rgss3` | The source of `vkrgss-guest-main` compiled over `vkrgss-binding-rgss3` and Ruby 1.9.2 | guest |
| `vkrgss-guest-rgss12` | The guest executable of games of RGSS1 and RGSS2: the guest role and `vkrgss-guest-main`, nothing of the host | guest |
| `vkrgss-guest-rgss3` | The guest executable of games of RGSS3: the guest role and `vkrgss-guest-main-rgss3`, nothing of the host | guest |
| `vkrgss-compose-ref` | CPU reference compositor | host and tests |
| `vkrgss-vk` | Vulkan backend: binning, compositing, presentation, pacing | host |
| `vkrgss-audio` | Mixer, decoding, MIDI synthesis | host |
| `vkrgss-host` | Event loop, input mapping, thread orchestration | host |
| `vkrgss` | Executable: command line, configuration files | |

`vkrgss-core` does no file I/O and depends on neither Ruby nor Vulkan.

## Ruby

- Embedded interpreter: vendored Ruby 1.8.7-p374 source under `third_party/ruby-1.8.7/`, with
  its own licence files, built by `vkrgss-ruby-sys`.
- Required flags: `-std=gnu89 -fwrapv -fno-strict-aliasing -fPIC`. Without the first two the
  build succeeds but is wrong (misdetected configuration, broken Bignum overflow), so Ruby's own
  core test suite (625 tests) is part of the merge gate.
- The vendored tree is our fork. Differences from the original's Ruby 1.8.1 found by oracle probe
  scripts are patched back when a game could observe them.
- Exposed surface matches the original: core library only, plus `Zlib` (extension compiled in,
  `Zlib.crc_table` fixed for 64-bit, zlib linked statically) and a `Win32API` shim. Load path
  cleared and `$KCODE='U'` at start.
- Regex: the original uses Oniguruma, and so does vkrgss: Oniguruma 5.9.6 is vendored and built
  in place of the interpreter's own engine (change `embed-oniguruma`); a probe of 65 patterns
  gives the original's results.
- The interpreter runs only on the script thread, with a large dedicated stack
  (`RUBY_INIT_STACK` at thread entry, `ruby_set_stack_size()` after init). Host signal handlers
  are restored after `ruby_init`. FFI declarations are hand-written.
- `system`, backticks, `exec`, `fork`, `syscall`, `IO.popen`, `Process.kill`, and `open`,
  `IO.read`, `IO.foreach`, `IO.readlines` and `IO.binread` with the name of a command (one
  that starts with `|`), and similar are stubs by default (log only), with a configuration
  switch. `require` and `load` only load `.rb` files inside the game directory, and
  `autoload` only what `require` may. `require` records a file once it has loaded, as Ruby's
  own does, and `load` with `wrap` evaluates the file in an anonymous module.
- `Win32API`: the class exists; common calls are emulated from a table; unknown calls log once
  and return 0; preload scripts may override.
- vkrgss-specific extensions live under a `VKRGSS` module only.
- The built-in Enterbrain Ruby source (`RPG::Sprite`, `RPG::Weather`, `RPG::Cache`, the `RPG::`
  data classes) is never committed. A tool extracts it from the user's own DLL into a
  git-ignored file that is loaded at run time.

### Floats in saves

Decided on 2026-10-15. A save holds floats that `Math` and `Float#**` computed, and their
results depend on the C library of the machine (glibc's version today, the operating system
later). Saves are to be identical byte for byte across machines and platforms, so both
interpreters take these functions from CORE-MATH, vendored as C (MIT), correctly rounded. A
pure Rust correctly rounded library would be preferred, but none covers the functions with
that guarantee today. Before it is vendored, its coverage of every function Ruby's `Math`
uses (`lgamma`, `tgamma`, `erf` and `erfc` included) is checked; a gap is decided then. Where
the oracle shows that a game depends on a result of the original that differs from the
correctly rounded one, that function alone is matched to the original.

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
- Fonts: `fontdb` scans font directories, with no fontconfig matching, plus a substitution
  table. The directories of `font_dirs` (configuration or command line) come first, then the
  game's `Fonts` directory and for RGSS3 the `Fonts` directory of each run-time package, then
  the system's: the `<dir>` entries of the fontconfig configuration files `fontdb` reads
  (`FONTCONFIG_FILE`, or `$XDG_CONFIG_HOME/fontconfig/fonts.conf`, `/etc/fonts/local.conf`
  and `/etc/fonts/fonts.conf`), or, when they name none, `/usr/share/fonts`,
  `/usr/local/share/fonts`, `~/.fonts` and `~/.local/share/fonts`. No CJK font is bundled.
- SoundFont: configured path, then `/usr/share/soundfonts`. Decided on 2026-10-15: the
  package carries GeneralUser GS with its licence as the default; a user's SF2 replaces it;
  DLS (`gm.dls`, the bank of the original's synthesiser) may follow.
- Display: no HDR output, because the picture would no longer look like the original's.
  winit already opens a native Wayland window; it gains the content type "game" (variable
  refresh, lower latency), fractional scaling (`wp_fractional_scale_v1`) and server-side
  decorations (`xdg-decoration`). X11 stays supported.
- Linux packages: Flatpak (Flathub) and AUR beside the tarball, which is built against an
  older glibc. Flatpak only if the guest's Landlock and seccomp work under Flatpak's own
  seccomp filter; otherwise AUR alone.
- The original's 10-second hang watchdog is off by default.
- Command line: `vkrgss <game directory>`.

## Feature tiers

- First version: scaling filters, accurate pacing, preload scripts, fast-forward, input record
  and replay.
- Reserved in the design, not implemented: higher internal resolution, post-process filters
  (planned route: RetroArch slang presets via `librashader`, to be verified), frame
  interpolation, widescreen through a run-time logical resolution.
- Added after the first version: save states (F3 and F4), see "Guest / host split".
- Not planned: rewind, HDR output.

## Process

- Workspace: `~/Desktop/rgss/` holds `ref/` (installer, original DLLs, unpacked DLL, RTP,
  tools), `games/` (local test games), `vkrgss/` (this repository), `wt/` (worktrees).
  Nothing under `ref/` or `games/` is ever committed.
- Everything in the repository is English. Rust edition 2024, toolchain pinned.
- `unsafe` only in `vkrgss-vk`, `vkrgss-ruby-sys`, `vkrgss-ruby192-sys`, `vkrgss-binding` (the source
  that `vkrgss-binding-rgss3` compiles too), `vkrgss-guest` and platform code.
  `thiserror` in libraries, `anyhow` in the binary and in `vkrgss-guest-main`, which is the
  body of the `main` of a guest executable, `tracing` for logs.
- Merge gate: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
  `cargo doc` with warnings denied, `openspec validate`.
- CI (decided 2026-10-15): the merge gate of CI keeps the fast checks, about ten minutes in
  parallel jobs, the fast part of the collector stress included. The rest of the stress,
  the sanitizers and fuzzing run in the nightly workflow, which opens an issue when it
  fails. `cargo xtask gate` stays one command that runs everything locally. No CI runner on
  the owner's devices.
- OpenSpec proposals are drafted interactively and reviewed by the owner. Each change is
  implemented in its own branch and worktree by an actor/reviewer flow that the owner launches.
  At most three non-overlapping module changes run in parallel.
- Oracle: the original `Game.exe` under Wine. Committed tests use only procedurally generated
  images; screenshots containing RTP or game art stay local.
- Reverse engineering uses Ghidra headless; there is no restriction on reading the original DLL.

## Several runtimes

Decisions of the owner of 2026-10-12, recorded by the change `extend-scope-to-vx-and-vx-ace`.

### Order and milestones

- VX first, then VX Ace. The open items of XP (text with the fonts of Windows, a long game
  played through by a person) go on beside this and neither waits for the other.
- Milestones of VX:
  - V0: the original runtime of VX starts under Wine, takes a schedule of input, and gives
    the same captures at the same frame numbers in repeated runs; how waits and fades of VX
    count as frames is established. This is a proof of the original's side only, done by
    the owner's coordinating session directly; the oracle tool may be adapted for it (frame
    size, the probe script). No change that implements VX in the guest is drafted before it
    has passed. Comparing with vkrgss begins at V1.
  - V1: title screen and menu.
  - V2: walking on a map.
  - V3: battle, saving and loading.
- A milestone is reached when the games of the corpus are identical to the original on
  compared frames that show that milestone's outcome, and for V3 also in a save written at
  the same frame. Before a milestone is declared reached, the record names the games and
  their revisions, the DLL of each, the seeds, the input schedules, the compared frames and
  the points at which saves were written, and that a repeated run gave the same.
- Audio, configuration, the sandbox and the save states get no milestones of their own.
  Their contracts apply to VX as they are, and are checked where the runtime behaves
  differently or the guest holds new kinds of resources (a state must bring them back; the
  music resumes differently after a music effect in VX). A measured difference in audio is
  implemented or recorded as an accepted limit.
- Version 0.2.0 is released when V3 is reached; it does not wait for VX Ace.

### VX Ace

Decisions of the owner of 2026-10-13, recorded by the change `plan-vx-ace`. That day the
original runtime of VX Ace was measured under Wine (the row A0 of "Milestones") and the
embedding of Ruby 1.9.2 was studied; both studies are kept outside the repository with their
tools and logs. The first two decisions were taken by choice among laid-out options, the
others by accepting the direction of the studies under the goal the owner set that day: VX
Ace finished and published.

- Milestones of VX Ace:
  - A0: the original runtime of VX Ace starts under Wine, takes a schedule of input, and
    gives the same captures at the same frame numbers in repeated runs; how its waits and
    fades count as frames is established. As V0, a proof of the original's side only. Done
    on 2026-10-13, with limits.
  - A1: title screen and menu.
  - A2: walking on a map.
  - A3: battle, saving and loading.

  The standard is that of "Goal and scope", and the rule of when a milestone is reached and
  what its record names is that of V1 to V3 in "Order and milestones". Version 0.3.0 is
  released when A3 is reached.
- Guest executable: a game of RGSS3 runs in a guest executable of its own,
  `vkrgss-guest-rgss3`, beside `vkrgss`: the binding over Ruby 1.9.2 and the guest role, no
  window, no Vulkan, no audio. This keeps the two interpreters' symbols and global state
  apart (868 global symbols are defined by both). The program knows the runtime before it
  starts a guest and starts that executable instead of itself, found beside its own path;
  when the guest attaches, the program checks that it is of its own version. The release
  archive holds both files.
- `--guest thread` with a game of RGSS3 is refused with a message that names
  `--guest process`. Without the option a headless run of RGSS3 uses a process.
- Interpreter: ruby-1.9.2-p0, the release the original embeds (it says `ruby 1.9.2p0
  (2010-08-18 revision 29036) [i386-mswin32_90]`), is vendored under
  `third_party/ruby-1.9.2/` as 1.8.7 is: a subset with a manifest, a patch log and a
  checked-in `config.h`, built without `configure` or a bootstrap Ruby by a sys crate of its
  own, `vkrgss-ruby192-sys`, which offers the Rust API of `vkrgss-ruby-sys`. It needs
  `-std=gnu89 -fwrapv`. It is our fork, patched to the behaviour measured on the original,
  as was done with Ruby 1.8.7. The stress run of the collector is part of the gate for it
  from the start.
- Binding: one source for the RGSS classes. `vkrgss-binding` is compiled a second time, as a
  package of its own over `vkrgss-ruby192-sys`; what differs by interpreter is behind the API
  of the sys crates or a runtime check. There is no second copy of the classes. The package
  is `vkrgss-binding-rgss3` (change `build-the-binding-over-ruby-1-9-2`): it has no source,
  runs the tests of `vkrgss-binding` a second time, and names in an allow-list those that
  expect RGSS1, RGSS2 or Ruby 1.8. A string the classes create is bytes, ASCII-8BIT over
  Ruby 1.9.2 as measured on the original, unless it is script source or the path of the
  script archive. Symbols are not numbered over Ruby 1.9.2: a dump of Ruby 1.9 does not
  depend on the numbers.
- Encodings: only what the original has is linked and listed, and an encoding the original
  does not know is unknown in vkrgss too. The original lists four (ASCII-8BIT, UTF-8,
  UTF-16LE, US-ASCII), with UTF-8 as the default external and source encoding; which
  encodings exactly, transcoding included, is measured on the original before the change
  that implements it.
- Hash seed: `String#hash` gets a fixed seed in vkrgss. The original's differs in every
  process, so there is no value to reproduce, and a run of vkrgss stays reproducible.
- Script threads: Ruby threads of RGSS3 are native threads. They run; a run with a live
  script thread is not claimed to be reproducible, and a save state is refused while one is
  alive. The timer thread of the interpreter is not started before a script makes a thread.
- Word size: the guest stays a 64-bit build against a 32-bit original. Differences a script
  can see (`2**30` is a Fixnum, `1.size`) are recorded as known differences unless a game
  of the corpus depends on one.
- Order of work: vendoring and the sys crate, then the binding, are one chain. Beside it,
  at once: the guest role as a library, runtime detection with the archive `Game.rgss3a`
  and the built-in scripts, and the parts of the RGSS3 surface that live in `vkrgss-core`.
  Behaviour of the RGSS3 classes is measured on the original before it is implemented, as
  for VX.
- The original side of every comparison runs without GPU device nodes and without network
  (change `run-the-original-without-device-nodes`).
- Corpus: Crysalis (unencrypted, DLL 3.0.0.1, the main test game), In Search of Immortality
  (`Game.rgss3a`, DLL 3.0.1.1, resizes its screen to 640x480) and Capella's Promise 1.09
  (`Game.rgss3a`, DLL 3.0.1.1). Each ships its DLL in `System/`; the run-time package holds
  none. A game is compared with the DLL it ships, as for VX.
- Tilemap, where the original reads outside its arrays (measured on 2026-10-13 on the DLL
  versions 3.0.0.1 and 3.0.1.1, recorded by the change `draw-the-tilemap-of-rgss3`): vkrgss
  reads nothing outside its tables.
  - The tile numbers 1024 to 1535 draw nothing. On the original 1024 to 1244 drew nothing
    and a page with 1245 to 1535 ended both DLLs with a page fault.
  - Negative tile numbers draw nothing.
  - A layer that a map lacks is empty, the shadow layer among them, so a map of three
    layers draws without shadows; layers beyond the fourth are not read.

  Known differences that follow: the five multiples of -256 from -256 to -1280 draw the
  block at (0, 0) of A5, A4, A3, A2 and A1 on the original and nothing in vkrgss; a map of
  fewer than three layers (two layers, one layer, two dimensions, one dimension) draws its
  layers in vkrgss where the original draws other things, differently on the two DLLs, or
  faults, and where the original faults vkrgss goes on. A map of three layers drew on the
  original as with an empty fourth, as it does in vkrgss.
- The process of this work is in "Process" below.

Text and bitmaps of RGSS3, recorded by the change `draw-text-and-bitmaps-as-rgss3` after the
measurement of 2026-10-13 on the DLLs 3.0.1.1 and 3.0.0.1 under Wine 11.0 (kept outside the
repository). The first decision may be overruled by the owner.

- DLL version: vkrgss follows 3.0.1.1, and the glyph rule of 3.0.0.1 is selected by the DLL
  the game ships.
  - The two versions differ in two things only. Where glyphs overlap in a mask, 3.0.1.1
    lets a later glyph replace earlier values where its own value is above 0, and 3.0.0.1
    over its whole black box, zeros included, so that a glyph erases what its left
    neighbour drew inside its box. And `Color#==` with an object that is no `Color` returns
    false on 3.0.1.1 and raises `TypeError` on 3.0.0.1.
  - 3.0.1.1 is the default because it is the later release, two of the three games of the
    corpus ship it, and the overwrite of 3.0.0.1 is a fault its publisher removed. The rule
    of 3.0.0.1 is built all the same, because the main test game ships 3.0.0.1 and a game
    is compared with the DLL it ships. It shows only where the box of a glyph reaches into
    its left neighbour's ink (italic styles, a few pairs of regular letters), and text
    glyphs are outside the pixel-exact standard with either rule.
  - The version is read from the version record of the DLL (`VS_FIXEDFILEINFO`, found by
    its signature), not from the file name: one game of the corpus ships 3.0.1.1 as
    `RGSS300.dll`. A hash would know only the files that were measured.
  - The table from version to rule:

    | DLL of the game | Glyph rule |
    |---|---|
    | a version whose first three numbers are 3.0.0 | that of 3.0.0.1 |
    | every other version | that of 3.0.1.1 |
    | a file without a version record | that of 3.0.1.1 |
    | no DLL | that of 3.0.1.1 |

    Applying the table is the guest's work (change `bind-font-and-text-of-rgss3`). A guest
    of RGSS3 reads the version record of the file its `Library` names, found in the game
    directory as a data file is and never in the archive, before the first script, and
    sets the glyph rule of its text engine by the table; an empty `Library`, a file that is
    not there or cannot be read and bytes without a record are "no DLL", with one line of
    the log that says why. Nothing of it is in the guest record: the guest reads `Game.ini`
    and the game's files itself, and the sandbox does not restrict reading. `RGSS_VERSION`
    follows the same table: `"3.0.0"` for a version whose first three numbers are 3.0.0,
    `"3.0.1"` in every other row.
  - Known difference: `Color#==` of 3.0.0.1 is not followed on any DLL. vkrgss returns false,
    as 3.0.1.1 does; raising where a comparison is false is a fault no script can rely on
    without rescuing it.
- Largest bitmap of RGSS3: the limit vkrgss has for every bitmap, 268435456 pixels (1 GiB of
  pixels), the limit of the compositors' bitmap stores, and no limit of 4194304 pixels as in
  RGSS2. The original has no rule to reproduce, only the free address space of a 32-bit
  process: 16384 by 16384, exactly this limit, was created, 32768 by 32768 and 20000 by
  20000 were not. Every size the measurement saw created is created, and a bitmap that
  could not be uploaded is refused before memory is taken. Known difference: the original
  created 8192 by 44954 in one run, which vkrgss refuses.
- What stands for the answers of GDI. vkrgss does not reproduce the glyph shapes of GDI; it
  holds layout, advances, placement and blending to the original, and supplies itself what
  GDI answers there. All of these were Wine's answers in the measurement; none was measured
  on Windows.
  - The glyphs GDI measures: each glyph hinted at the whole pixels per em as Wine's
    FreeType hints it for an anti-aliased target without load flags, run by `skrifa`. The
    engine is FreeType's choice: the automatic hinter for a TrueType font without `fpgm`
    whose `prep` is at most 7 bytes (VL Gothic), the font's own engine otherwise (the
    TrueType interpreter for VL PGothic, the PostScript hinter for CFF). `skrifa`'s own
    choice takes the interpreter for any `prep` and is not used. Measured: the rule below
    gives all 28392 values of the `text_size` table of VL Gothic and VL PGothic, sizes 6 to
    96 in four styles, where the unhinted advances gave 22658.
  - The advance of a glyph (`gmCellIncX`): the hinted advance rounded up to whole pixels.
    In a font whose OS/2 panose proportion is monospaced (9), with `a = xAvgCharWidth` and
    `s = MulDiv(ppem, 65536, upm)`, a glyph whose advance equals `MulFix(2 a, s)` is
    `2 MulFix(a, s)` wide, as Wine's GDI makes it (`あ` of VL Gothic 18 at size 24, where
    the hinted advance is 19).
  - The left-bearing correction: the left overhang of the first glyph. The original also
    moves the pen when a later glyph would start left of column 0; vkrgss does not. Accepted
    limit.
  - `tmHeight` and `tmAscent`: the rounded ascent plus the rounded descent, and the rounded
    ascent, at that pixels per em. The decrement in the choice of pixels per em stays a
    placeholder of Wine.
  - The box of a glyph (`gmptGlyphOrigin.x`, `gmBlackBoxX`): the control box of the hinted
    outline on whole pixels outwards. The left overhang of the first glyph is minus its left
    edge, the right overhang of the last glyph its right edge beyond its advance, each when
    positive.
  - Bold without a bold face: every advance one pixel wider. The original adds 1 per UTF-16
    unit and vkrgss 1 per character, which is the same inside the Basic Multilingual Plane;
    outside it, accepted limit. The box is that of the hinted outline after vkrgss's
    emboldening, not on whole pixels; the strength of the emboldening stays a placeholder of
    Wine's.
  - Italic without an italic face: the regular advances, and the four corners of the box
    slanted by `x + y / 4`, the left edge rounded down and the right edge rounded up.
  - The average character width (`tmAveCharWidth`): `MulFix(xAvgCharWidth, MulDiv(ppem,
    65536, upm))`, at least 1, plus 1 for a synthesised bold (9 for VL Gothic at size 24).
    In RGSS3 it enters only the squeeze.
  - The glyph images: rasterised from the same hinted outlines, emboldened, slanted and
    condensed as before, so that ink lies inside the box and the pen places it as GDI does.
    The shapes still differ from GDI's.
  - RGSS1 and RGSS2 keep the unhinted advances of the face in whole pixels and their
    box model, although the rule is Wine's GDI and so would hold under Wine for them too:
    for the fonts they were measured with it changes no value (Noto Sans CJK SC is a CFF
    font, whose hinter changes no advance beyond that rounding; the hinted advances of
    Liberation Sans equal the rounded ones at every size from 4 to 80 pixels per em). A
    font that FreeType auto-hints or that is monospaced by its panose may give other widths
    under Wine in RGSS1 and RGSS2; that needs a measurement of its own.
  - The 65 levels of coverage: the coverage `c` of the rasteriser, 0 to 255, becomes the
    level `c * 65 / 256`, and the mask holds `trunc(level * 255 / 64)`. The second step is
    measured; the first is a placeholder for GDI's own 65-level answer.
  - The width a font gets for an `lfWidth`: the model vkrgss has for a font of a given
    average width. Accepted limit: today's Wine gives other widths for one measured font,
    and the model needs a measurement of its own for all three runtimes before it changes.
  - The average character width and the height of the System font, by which RGSS3
    squeezes: the constants 7 and 16, Wine's values. A Windows of another language has
    another System font.
  - The font chosen for a missing name: the substitution table and the fallback families.
- Fonts of a game of RGSS3 (change `bind-font-and-text-of-rgss3`): the directories of
  `--font-dir`, the game's `Fonts` directory, then the `Fonts` directory of each run-time
  package of the game in the order of the packages, each when it exists, then the system's
  fonts. The original loads the fonts of `<RTP>/Fonts` and `<game>/Fonts`; the game comes
  before the packages as in the search of the game's files. RGSS1 and RGSS2 keep their
  list. "VL Gothic" gets no entry of the substitution table: a game of VX Ace names the
  package or ships its fonts, and without either the fallback families draw.

The window of RGSS3, recorded by the change `draw-the-window-of-rgss3` after the window
measurement of 2026-10-13 on the DLLs 3.0.1.1 and 3.0.0.1 under Wine (kept outside the
repository; 1185 captures, the same on both versions). The window is in `vkrgss-core`; its
class comes with the binding over Ruby 1.9.2.

- Where the original ends with a page fault, vkrgss goes on. Known difference: the original
  ends, vkrgss does not.
  - The calls: `ox=`, `oy=`, `padding=`, `padding_bottom=` and an assignment of
    `cursor_rect` that leaves its size 0 by 0, on a window on which `windowskin=` was never
    called and whose `cursor_rect` never changed its size. In vkrgss they take effect by the
    measured rules: the cursor is placed, and nothing is drawn of it without a skin.
  - The sizes: a window of 20000 by 20000 with a skin, a window of 100000 by 100000, a
    `cursor_rect` of 100000 by 100000. In vkrgss a window or a cursor too large for a bitmap
    has no built background, frame or cursor, and its other parts are drawn.
- A window whose width or height is 0 or less draws no background and no frame, and its
  cursor, arrows, pause sign and contents are cut to the one pixel at its origin. This rule
  rests on 14 measured pixels, 9 it was fitted to and 5 of the held-out sets, and was not
  examined with readable bitmaps.
- Skins smaller than the layout: the measurement found the rule the measurement of RGSS2
  had left open (a stretched block that leaves the skin is cut and its target shortened).
  The captures of VX decide it for the background alone, where it removes every wrong
  pixel, so the background of a window of RGSS2 follows it too.
- Not measured for the window of RGSS2 on 2026-10-13, and left as it was there, each
  needing a measurement on the original of VX first:
  - the cursor from a skin that cuts the cursor block, and the rule of the odd column for
    an arrow or pause sign that the skin cuts (in RGSS3 both are measured and followed);
  - the list of calls that decide the scroll arrows: RGSS3 decides them at `width=`,
    `height=`, `ox=`, `oy=`, `arrows_visible=`, `move` and the assignment of another skin or
    contents object, and not in `update`; the report infers the same for RGSS2 from
    captures that do not tell the two apart, and RGSS2 keeps deciding them in every
    refresh;
  - a disposed skin or contents bitmap (in RGSS3 both are drawn as none).

Sprites, planes, viewports, snapshots, the frozen picture and input of RGSS3, recorded by
the change `draw-sprites-and-snapshots-as-rgss3` after the measurement of 2026-10-13 on the
DLLs 3.0.0.1 and 3.0.1.1 under Wine (`ace-v1-sprite-REPORT.md` with its model and frames,
kept outside the repository; the two versions gave the same results). These rules are in
`vkrgss-core`, `vkrgss-compose-ref`, `vkrgss-proto`, `vkrgss-host` and the presenter of `vkrgss-vk`;
the classes and modules of RGSS3 that use them come with the binding over Ruby 1.9.2.

- The rules of a scene and of an input state have a value for RGSS3, `Rules::Rgss3`, beside
  those of RGSS1 and RGSS2. The window and the tilemap of RGSS3 keep rule sets of their own
  and live in a scene of any rules. RGSS3 equals RGSS2 wherever the measurement of
  2026-10-13 found no difference (275 of the 327 frames of the probes of VX are byte-identical on RGSS3): pixel
  arithmetic, zoom, bush rows, wave, flash, the tile of a plane, the tone, colour and flash
  of a viewport, the order by z, y and creation, the repeat timing of `Input`, brightness,
  fades and transitions. It changes two things:
  - A turned sprite in a viewport stands at its own position plus the viewport's rectangle
    position less the viewport's origin, and its bounding box and its texels both come from
    that position, as for a sprite on the screen. RGSS1 and RGSS2 keep the shift between
    the two (359 frames without a wrong pixel; the rule of RGSS2 leaves 213035 wrong).
  - When an input snapshot is seen, below.

  Until the binding over Ruby 1.9.2 makes the scene and the input state of a game of RGSS3
  with this value, `vkrgss-binding` gives such a game the rules of RGSS2.
- When an input snapshot is seen (measured on 2026-10-13). The original takes the window messages of keys only in
  the calls of `Graphics` that show frames; `Input.update` takes none. vkrgss has no queue of
  key messages: the host sends whole input states. With the rules of RGSS3 an update
  latches the most recent snapshot that had arrived when a frame was last shown, and a
  snapshot that arrives afterwards waits for the next shown frame. The input state has a
  call that marks a shown frame, which the binding of RGSS3 is to make once for every frame
  it shows and from nowhere else. So `Input.update` alone never sees a new key, and a key change before a
  `Graphics.update` is seen by the `Input.update` after it, as measured (240 of 240). A
  scheduled or replayed run gives the answers it would give under the rules of RGSS2,
  because the state of a frame arrives ahead of that frame's tick. Known differences:
  - The count of key messages a frame takes. The original takes exactly one in a frame
    that has no time to wait and more, up to all that are queued, in a frame that waits;
    how many in between is not a rule, so vkrgss imitates no count: every shown frame makes
    the whole state as it arrived visible. Several key changes between two frames are seen
    together, where a frame of the original without slack spreads them over the following
    frames.
  - A key pressed and released between two frames is never seen, where the original saw it
    for one update when the frame took only the key-down (2 of 5 in one measurement, 0 of
    16 in clean loops).
  - The message the original takes now and then during `Kernel#sleep` is not imitated.
- The frozen picture (measured on 2026-10-13). The original of RGSS3 keeps it in one buffer of 640 by 480 pixels,
  black at the start of the process: `Graphics.freeze` copies the screen, at its size of
  that moment, into the top left corner and leaves the rest, and a transition blends buffer
  pixel (x, y) with the new scene for every pixel of the screen as it is then; nothing is
  scaled or centred, and frames merely shown do not enter the buffer (13 screenshots
  without a wrong pixel, plain transitions only). The guest names the rule with the freeze
  message, as it names the rule of a transition (protocol version 14), because the host
  does not know runtimes; the presenter keeps the buffer. RGSS1 and RGSS2 keep the held
  frame at its own size: whether RGSS2 has such a buffer was not measured. Decided without
  a measurement:
  - While frozen after a resize the display shows the buffer's pixels for the screen as it
    is now; the window of the original shows stale contents until the next frame.
  - A masked transition after a resize tiles the mask over the screen as it is now and
    takes the old picture from the buffer.
  - A frame wider than 640 or higher than 480, which the original cannot have, is frozen as
    a frame of its own size.
- A snapshot of RGSS3 (`Graphics.snap_to_bitmap`) is the composed scene as the objects are
  now, darkened by the brightness, with an alpha of 255, and all of (0, 0, 0, 0) at
  brightness 0 (256 snapshots without a wrong pixel). RGSS2 keeps an alpha of 255 at
  brightness 0, where it was not measured. One function of the reference compositor makes
  both; `Graphics.snap_to_bitmap` of RGSS2 uses it, and that of RGSS3 comes with the
  binding.
- Where the original faults or reads outside a table (measured on 2026-10-13), vkrgss goes
  on. Known differences:
  - A disposed viewport given to a move (`Sprite#viewport=`, `Plane#viewport=`): the
    original ends. The scene of vkrgss refuses the move with a disposed error and moves
    nothing, for every runtime; what the classes of RGSS3 raise from it is decided with the
    binding.
  - The sprite that ends the original: one sprite (bitmap 20 by 90, `src_rect`
    (8, 38, 9, 26), zoom 2.5 by 0.7, angle 180, mirrored, `bush_depth` 20, at (434, 160))
    ends it inside `Graphics.update`; the condition was not found. vkrgss draws it by the
    rules of a turned sprite. There is no picture of the original to compare with.
  - A negative wave length: the original reads outside its sine table, and its two versions
    show different pictures. vkrgss takes the angle of the wave modulo 360, as its sprite of
    RGSS2 does.
The guest executable and the start of RGSS3, recorded by the change
`run-rgss3-games-in-a-guest-of-their-own` on 2026-10-13. The behaviour of the start is that
measured on the original runtime 3.0.1 under Wine that day (the measurements are kept
outside the repository); what of it is not followed is named as a known difference.

- Where the guest executable is: `vkrgss` starts `vkrgss-guest-rgss3` from the directory of its
  own file, links resolved as the system resolves them. When the file is not there, the run
  ends before a guest, a window or a device starts, with an error that names the file and
  the directory. No option, variable or configuration key names another place. Both
  executables have the version of the workspace, and each refuses a record or a connection
  of another version. Each executable runs the runtimes of its interpreter only: Ruby 1.8.7
  those of RGSS1 and RGSS2, Ruby 1.9.2 those of RGSS3.
- How tests reach it: the tests of the program that run a game of RGSS3 are a file of their
  own and let the program find the guest executable beside it, where cargo puts every
  binary of the workspace; `cargo test --workspace` and `cargo test -p vkrgss -p
  vkrgss-guest-rgss3` build it first. A test that finds none fails and names that command.
  There is no artifact dependency, no build script that builds the other package and no
  variable that names the file.
- The form of the guest: a game of RGSS3 has its guest in a process, headless too. A choice
  of `thread` ends the run with a message that names `--guest process`.
- Experimental: VX Ace is experimental while its games start and the milestones A1 to A3
  are open. A run of RGSS3 says so once, as a warning of the program; the documents say so,
  and the rows A1 to A3 stay "Not started". This ends when A3 is reached.

  A3 was reached on 2026-10-14, and with it this state has ended: VX Ace is supported, a run
  of RGSS3 no longer says that its support is experimental, and version 0.3.0 is the release
  that "Milestones of VX Ace" above names (change `release-0-3-0`).
- Preparation: chosen by the runtime of the run. A game of RGSS3 gets a prelude of its own
  in the place of that of RGSS1 and RGSS2: `RGSS_VERSION` by the version of the DLL the
  game ships, `"3.0.0"` or `"3.0.1"` by the table of the glyph rule under "Text and bitmaps
  of RGSS3", `RGSSReset < Exception`, `save_data`, `load_data`, `msgbox` and `msgbox_p`, and no `Reset`, `Hangup`,
  `print` or `p`, which are Ruby's own there and write to the standard output. `rgss_main`
  and `rgss_stop` belong to the built-in scripts file, which is evaluated whole before the
  script archive is read. `$TEST`, `$BTEST` and `$DEBUG` are set as for RGSS2. The seed is
  set once, before the built-in scripts. Beside the functions that start processes of
  RGSS1 and RGSS2, `spawn`, `Process.spawn`, `Process.exec` and `open` of a command are off
  unless the caller enables them. The binding over Ruby 1.8.7 refuses a game of RGSS3.
- Script loading: `$RGSS_SCRIPTS` is what `load_data` returns for the script archive, each
  entry with its inflated source inserted at index 3; the count is read once, and entry `i`
  is evaluated on the main object under the name `{%04d}` from `$RGSS_SCRIPTS[i][3]` as it
  is when its turn comes, compiled as UTF-8 from a new string of its bytes. The filesystem
  encoding of the interpreter is ASCII-8BIT, as on the original, so `__FILE__` is bytes.
  Known difference: an archive that cannot be loaded is an error that names it with a
  failure status, where the original shows `Failed to load script.` and ends with code 0.
- Data files: `load_data` is `Marshal.load` of the bytes of the file with a proc that
  labels every String of ASCII-8BIT as UTF-8, called through Ruby; `save_data` writes the
  dump and returns `nil`. Known differences: a frozen string the original gives in two of
  71 samples, the errors of a loose file, and the lookup rules of the original.
- Reset: a reset request of the host makes the next `Graphics.update` raise `RGSSReset` with
  an empty message. `Graphics.__reset__` disposes every bitmap and scene element, sets the
  brightness to 255 and the frame rate to 60 and ends a freeze; `Audio.__reset__` stops the
  audio. An `RGSSReset` that leaves a script calls `Graphics.__reset__` through Ruby, stops
  the audio, clears the input and runs the scripts again from the first, as for the other
  runtimes. Known differences: a raise that arrives without a call of `Graphics`, the
  waits and fades cut short, and `__reset__` waiting for the release of F12.
- Endings and the report: `exit` in any form, `raise SystemExit`, `abort` and the end of
  the last script end the game with status 0. An uncaught exception ends it with status 1
  and a report in the measured form: `Unable to find file:` for `Errno::ENOENT`, `Script
  '<name>' line <n>: <class> occurred.` with the message, or `<class> occurred.` without a
  line; the position is the first backtrace entry outside `(eval)`, the name is
  `$RGSS_SCRIPTS[i][1]` as it is then, and a `SyntaxError` takes its position from its
  message. Known differences: the original's exit code, which follows the line number in
  most cases, the empty box for a text of more than 1023 UTF-16 units, `at_exit` handlers,
  which the original never runs and vkrgss does not either, and `exit!`.
- Message boxes and console output: `msgbox` joins `to_s` of its arguments (`nil` as `nil`)
  with `$,`, adds `$\` and cuts at a NUL byte; `msgbox_p` joins `inspect` with line feeds;
  both show a modal message, write the text to the error stream and return `nil`. `p`,
  `print` and `puts` are Ruby's and write to the standard output of the program.
- States with script threads: a state of a guest of RGSS3 is a copy made by `fork` as any
  other. While a Ruby thread other than the main one is alive no state is saved: the guest
  says so for the slot and the save counts as failed. A copy forgets the timer thread of
  the process it was copied from, and starts one when a script makes a thread. The sandbox
  is that of the other runtimes; the threads a script makes inherit it.
- Release archive: `cargo xtask dist` builds both executables in one build with libcrypt
  linked statically into both, refuses a shared libcrypt in either and packs them side by
  side with the licence texts of Ruby 1.9.2.
- The classes of a game of RGSS3 are those of RGSS2 still. The class surface of RGSS3 is
  bound by the changes that follow, in their own order. A title screen of the standard
  scripts calls of it, between the end of script loading and its first frames: `Tone.new`
  and `Color.new` without arguments, `Viewport.new` without arguments,
  `Window#initialize(x, y, width, height)`, `padding=`, `padding_bottom=`, `tone`,
  `Tone#set`, `Rect.new` without arguments, `Rect#set` and `Color#set` with one object,
  `open?`, `Audio.bgm_play` with four arguments, and soon after `Input.repeat?` with a
  Symbol.

Sprites, planes, viewports and `Graphics` of RGSS3, recorded by the change
`bind-the-scene-classes-of-rgss3` on 2026-10-13 after the measurements of that day (kept
outside the repository). The scene of a game of RGSS3 is made with `Rules::Rgss3` from this
change on; its input state keeps the rules of RGSS2 until the binding marks the frames it
shows. Decided without a measurement:

- Disposing a viewport of RGSS3 leaves its sprites, planes, windows and tilemaps alive, as
  measured. vkrgss draws them nowhere while they are in that viewport, by the rule of the core
  that hides the elements of a disposed viewport; they are not clipped to its last
  rectangle and not shown on the screen. Moved to the screen or to another viewport, they
  are drawn there. What the original draws of them was not measured; nothing measured
  supports drawing them anywhere. A disposed viewport given to `viewport=` raises
  `RGSSError` with `disposed viewport`, where the original ends. RGSS1 and RGSS2 keep
  disposing the elements with the viewport, although the original of RGSS2 keeps them alive
  as RGSS3 does.
- `Graphics.play_movie(name)` plays nothing. It looks for a file of the game directory under
  the name as given and then with `.ogv` appended, the extension the editor of VX Ace
  writes; the run-time packages and the archive are not searched. Found, it returns `nil`
  without a frame; not found, it raises `Errno::ENOENT` with the name as given, as measured.
  How the original looks a movie up and what it shows were not measured.

Audio of RGSS3, recorded by the change `bind-audio-of-rgss3` after the measurement of
2026-10-13 on the DLLs 3.0.1.1 and 3.0.0.1 under Wine (`ace-v1-api-REPORT.md` section 6,
kept outside the repository, with the script that fits the rule to 982 readings).

- The position is the guest's. Nothing flows from the audio of the host to the guest, and
  nothing may: the audio device runs on its own clock, a headless run has none, and what the
  guest does depends on its ticks alone. The original's value is not repeatable either: two
  runs of the original with the same input wrote saves whose two copies of `Audio.bgm_pos`
  are one chunk apart. Its rule is exact in its values and steps; when a step happens
  depends on real time.
- The probe: `vkrgss_core::sound::SoundFormat::probe` reads, from the bytes the guest has
  read, the sample rate, the channel count, the length in sample frames and the tags
  `LOOPSTART` and `LOOPLENGTH` (upper case only) of a WAV file of 8-bit or 16-bit PCM and of
  an Ogg Vorbis file; for everything else it gives nothing. It is written for this purpose:
  a decoder in the guest for four numbers was rejected, and so was asking the host.
- The model: `vkrgss_core::sound::Position` is the measured rule with time counted in frames
  shown. A sound of more than 529200 bytes of 16-bit PCM is streamed: its decoder starts at
  `pos`, reads 15 chunks of 187.5 ms at once and one more each time the frames shown, at
  their frame rate and the pitch of the sound, amount to 187.5 ms, and wraps at the loop
  end to the loop start; the position is the decoder's less 16 chunks, not below 0. A
  shorter sound is a cursor from `pos` that wraps at its end. Integers only.
- Which frames count: those the guest sends and waits a tick for outside a modal message,
  of `Graphics.update`, `wait`, `fadeout`, `fadein` and `transition`, as for
  `Graphics.frame_count`; setting `Graphics.frame_count` changes nothing of it. A music
  effect whose length is known holds the music for that many frames.
- What a save holds: the standard scripts store `Audio.bgm_pos` twice. For a streamed Ogg or
  WAV file vkrgss writes a value of the original's own sequence for that file and start, the
  original's own value at that frame when the original has read the same number of chunks,
  and otherwise one whole chunks away; the comparison of a save for A3 allows the four
  bytes of the two positions to be equal or apart by whole chunks, and its record says
  which.

### Several runtimes in one program

- One repository, and the name stays for now.
- One program. The RGSS version is a value known at run time; XP and VX share one build of
  the embedded interpreter. Where the interpreter itself behaves by runtime (the symbol
  numbers at start-up, which decide the order of instance variables in a save, are those
  of XP today), the runtime is chosen before the interpreter is initialised. What is
  measured for VX does not replace the behaviour of XP. The first change that implements VX
  says which recorded patches are shared and which are chosen by runtime.
- The requirements of the main specifications that state behaviour of XP (the default frame
  rate of 40, a viewport that disposes of its elements, `$DEBUG`) are contracts of RGSS1.
  A change that implements VX qualifies the requirements it touches by runtime and adds
  scenarios for RGSS2; the scenarios of XP stay. The requirements on the protocol, the
  boundary and the purity of the core apply to every runtime.
- The code of VX goes into the existing crates, told apart by a version value. Only the
  bindings of another interpreter get a crate of their own.
- The runtime of a game: an option of the command line decides when given. Otherwise it is
  taken from a recognised `Library=` of `Game.ini` and from the extension of the file that
  `Scripts=` names (`.rxdata`, `.rvdata`, `.rvdata2`), in a directory or in an archive
  alike; other files are not looked at. When the two disagree, or neither says anything,
  the run ends with a message that asks for the option.
- Starting a game is by runtime where it differs: the key `RTP=` of VX beside `RTP1` to
  `RTP3`, the archive `Game.rgss2a` beside `Game.rgssad` (the same encoding; the archive of
  VX Ace is another), and a file of built-in scripts for each runtime, extracted from the
  user's own DLL and never committed. These are settled before V1.
- The tile system of VX is an op kind that the CPU reference and the compute shader both
  evaluate from the start; no cache of composed maps comes first.
- The runtime gives the logical size of the frame and the frame rate at the start. The
  setters of RGSS itself (`Graphics.resize_screen`, `Graphics.frame_rate=`) change them
  within the limits of that runtime, and a reset restores the runtime's values. These are
  part of VX, not tricks.

### Corpus

- Both run-time packages are fetched from their publisher's pages into the reference
  directory outside the repository.
- At least three games for each runtime, by different authors: the owner's own first, then
  well-known free games, whose list the owner sees before anything is fetched. A game is
  kept outside the repository and only a copy of it is run: by vkrgss in the confined guest
  process, by the original under Wine, whose prefix is not that sandbox.
- What is built beyond the default scripts follows the corpus: it decides which further
  `Win32API` calls are emulated and which patches are written. The limit that games which
  depend heavily on Windows may need patches stays. A DLL that a game ships and that would
  have to be executed, a script that reads or writes the memory of the runtime, and a
  change of resolution made past the functions of RGSS each get a decision of their own on
  whether and how.

### Before VX is done

- VX work is merged to `main` change by change. A VX game is run as far as the code goes,
  and the run says once that support for VX is experimental.
- Until V3 the documents say: XP is supported; VX is experimental or planned, whichever is
  true at that time; VX Ace is planned.

V3 was reached on 2026-10-12, and with it this rule has ended: VX is supported, a run of a VX
game no longer says that its support is experimental, and version 0.2.0 is the release that
"Order and milestones" names (change `release-0-2-0`).

### Process

- As before: OpenSpec changes implemented by flows, at most three in parallel. The owner's
  standing go-ahead, which until 2026-10-13 ended at V3 and did not cover VX Ace, covers
  VX Ace through the release of 0.3.0, pushing for that release included (change
  `plan-vx-ace`).

## Platforms

Decided by the owner on 2026-10-15 (change `plan-the-platforms-and-modernization`). Linux on
x86_64 is supported. The planned platforms come in this order, each as its own set of
changes; finishing one does not commit to the next. Their milestones are P1 to P4.

### ARM Linux

- Accepted when, on aarch64, the frames of the seven XP games and of the VX and VX Ace games
  of the corpus at the compared frame numbers are identical byte for byte to those of
  x86_64, and a save written at the same frame is identical byte for byte. The original
  runtime does not run there, so x86_64 is the reference.
- Both interpreters get a verified `config.h` of aarch64 (Ruby 1.8.7 borrows the one of
  x86_64 today), and the collector stress runs on aarch64.
- The job `gate-aarch64` on GitHub's hosted `ubuntu-24.04-arm` with lavapipe becomes
  required.
- Hardware: the two Jetson Orin Nano of the owner (`duck-stable`, `duck-naughty`; JetPack 7.2,
  NVIDIA's Vulkan driver, no display) may be used for tests at any time, never as runners.

### macOS

- Apple silicon through MoltenVK. MoltenVK has no present timing, so pacing falls back to the
  timer; portability enumeration and the portability subset; the number of bitmap slots is
  taken from the device's limits.
- Prerequisite: the guest of RGSS1 and RGSS2 becomes a program of its own instead of the
  host executing itself again, so that no window framework is linked before `fork`.
- Tested only on the owner's MacBook Air, by hand, now and then, and only after asking the
  owner each time. No CI runner for macOS, hosted or self-hosted.
- A guest process on macOS is not confined: Landlock is Linux's, and nothing takes its place
  for now (change `build-and-test-on-macos`); a later change may add `sandbox_init`. The
  guest ends with its parent through a kqueue that waits for the parent's exit, as on Linux
  through the parent-death signal.
- The workspace builds for `aarch64-apple-darwin` and its tests pass there on MoltenVK
  (change `build-and-test-on-macos`), those kept to Linux apart, each with its reason. Both
  interpreters use configuration headers that `configure` wrote on the Mac, reviewed, and
  name the platform `aarch64-darwin`. A headless run composites on the Apple GPU by default,
  the Mac having no CPU rasteriser. P2 is not accepted with this: packaging, a window, audio
  output and the games of the corpus are not covered yet.
- No Developer ID is bought (the owner, 2026-10-21): a macOS release is the app signed ad hoc
  and not notarized, and the user removes the quarantine attribute of a downloaded copy
  (see running.md). Not in the Mac App Store.

### Android

- Starts with a measurement of feasibility, as V0 did: both interpreters on bionic, a
  separate guest process under seccomp, Vulkan 1.3 on the test phones. The scope is fixed
  after it.
- The minimum is Android 14, API level 34, decided by the owner on 2026-10-17 after the
  measurement of 2026-10-16/17 on both test phones: below it bionic has no public
  `__freadahead`, without which `IO#eof?` of Ruby 1.8.7 answers wrongly and
  `IO#readpartial` blocks, because bionic's `FILE` is opaque. Both interpreters build for
  `aarch64-linux-android` at that level and pass their tests on both phones (change
  `build-the-interpreters-for-android`, [building.md](building.md#android)). bionic has no
  `crypt`, so `String#crypt` raises `NotImplementedError` there.
- First version: a game is imported as a zip into the app's private directory, not reached
  through the storage access framework, because the runtime reads and writes by path; input
  through on-screen buttons and gamepads. Save states came with the guest in a process
  (change `run-the-guest-in-a-process-on-android`, 2026-10-17).
- Vulkan 1.3 is required as on the desktop; no path for 1.1.
- The guest is a separate process under seccomp (change
  `filter-the-system-calls-of-the-guest`, whose filter test passes on both phones). Most
  Android kernels have no Landlock; the private directory keeps the game's files apart.
- Distribution: an APK on the GitHub releases, installable through Obtainium, self-signed.
  Google Play may come later.
- Test devices: the owner's Pixel 7a (Tensor G2, Mali-G710) and Galaxy S24+ SM-S926B
  (Exynos 2400, Xclipse 940), on USB at the owner's desktop, free to use at any time; the
  Android emulator for routine tests.

### Windows

- Save states (F3 and F4) are never supported on Windows: the interpreter's state cannot be
  serialized, because the C stack holds the script's calls, and emulating `fork` is not
  supported by the system and breaks with its updates.
- Whether the guest is a 32-bit process (Ruby 1.8.7 needs `long` as wide as a pointer, which
  64-bit Windows does not give) is decided when the work on Windows starts. The owner has no
  Windows machine yet.
- Decided by the owner on 2026-10-17: Windows is not considered for now; P4 stays "Not
  started" and no work toward it is planned.

### Order of work

1. `fuzz-and-review-ruby-1-9-2`.
2. The correctly rounded library of "Floats in saves".
3. The merge gate of CI split into parallel jobs.
4. The guest of RGSS1 and RGSS2 as a program of its own.
5. ARM Linux, then macOS, then Android; Windows last.

The Vulkan items of the audit (synchronization validation in the tests,
`vkQueueSubmit2` with timeline semaphores, batched upload barriers, swapchain maintenance
in its KHR form, the swapchain's timing properties and variable refresh) and the Wayland
items go in between; they do not depend on a platform.

## Milestones

Status as of 2026-10-09. "Evidence" names what was checked; "Not covered" is what the status
does not claim.

The owner confirmed on 2026-10-09 that all milestones M0 to M6 are complete as recorded here,
accepting the limits listed under "Not covered". Work beyond them is new work. That
confirmation concerns M0 to M6 only. The rows V0 to V3 are the milestones of VX, decided on
2026-10-12 (see "Order and milestones"); a row is set to "Done" only with the record that
section asks for. V0 is a proof of the original's side alone and was done on 2026-10-12.
The rows A0 to A3 are the milestones of VX Ace, decided on 2026-10-13 (see "VX Ace"), under
the same rule; A0 is, like V0, a proof of the original's side alone and was done on
2026-10-13. A1 to A3 were done on 2026-10-14.
The owner accepted on 2026-10-13, before A1 to A3 were reached, that their compared frames
may differ from the original inside the rectangles of text, as the frames of XP and VX do:
glyph shapes are not compared (see "Goal and scope"); positions, widths and placement are.
The rows P1 to P4 are the platforms, decided on 2026-10-15 (see "Platforms"), under the
same rule. P1 was reached on 2026-10-16.

| | Outcome | Status | Evidence | Not covered |
|---|---|---|---|---|
| M0 | Workspace skeleton, guest/host protocol, oracle harness | Done | `cargo xtask oracle` compares a game frame by frame with the original runtime under Wine; used on eight game set-ups | |
| M1 | Title screen of the test game, navigable | Done | The test game has no title screen and goes straight to its map; its menu, and the title screens of two other games, are navigable and compared with the original | |
| M2 | Map: tilemap, viewports, planes, walking | Done | Map frames of the test game, a second large game and three more games are identical to the original pixel for pixel; tilemaps are evaluated by the compositor as decided | |
| M3 | Audio: Ogg, WAV, MP3, MIDI | Done | Decoding and mixing tests; the owner confirmed sound in a windowed run of a real game | What the original does with more than one MIDI loop marker; WMA without `ffmpeg` installed |
| M4 | Full play-through: battle, menus, save/load, F12 reset | Done | Battle course, menu and a save written at the same frame (byte for byte) equal the original; reset is covered by tests of vkrgss alone | A complete play-through of a long game by a person; F12 cannot be injected into the original for comparison |
| M5 | Modernisation: filters, present-timing pacing, configuration, gamepad | Done | Scaling filters, fullscreen, layered TOML configuration, gamepads, input recording and replay; `VK_EXT_present_timing` pacing runs on the AMD GPU at 60, 120 and 144 Hz with a mean interval of 25.0 ms | Whether present-timing pacing looks smoother than timer pacing on a physical display has not been judged by a person |
| M6 | Wider compatibility: encrypted archives, `Win32API` shims, more games | Done | A packed game is identical to the original on every compared frame with both runtimes reading only the archive; common `Win32API` calls and raw key state are emulated; 20 of 23 further XP games start and the three others stop on files missing from the projects | An archive written by the original packing tool; custom DLLs that games ship |
| V0 | The original runtime of VX under Wine: a schedule of input, the same captures at the same frame numbers in repeated runs, how waits and fades count as frames | Done | 2026-10-12, Visions & Voices (its DLL is version 2.0.0.1 under the name `RGSS202E.dll`), seed 1, a schedule of 11901 frames through title, name input, a battle, walking and the menu: 17 window captures and 9 snapshots byte for byte equal in two runs, and again with seed 20261009. The frame rules of `Graphics.update`, `wait`, `fadeout`, `fadein`, `freeze` and `transition` hold on 331 frames of a probe game with no wrong pixel, on that DLL and on version 2.0.2.1. The Witch's House starts from its archive with the probe inside | A Ruby thread ends the original under Wine, so the game was run with thread blocks executed at once; a frame inside a transition cannot be held, only the one after it; mask transitions were recorded, not fitted; audio; the third game of the corpus is not unpacked |
| V1 | VX: title screen and menu | Done | 2026-10-12, Visions & Voices with the DLL it ships (2.0.0.1), `cargo xtask oracle` with `--no-threads`, seed of the harness, the input schedule of V0 extended by the keys that open the menu: the title (frames 100 and 261, the second inside its fade-out) and the menu over its blurred snapshot (frames 12100 to 12550) equal the original outside the rectangles of text; frame 261 has no differing pixel | The owner accepted on 2026-10-12 that frames with text differ in the pixels of the glyphs. The harness does not leave the pixels of text out: every frame with text counts as different, and each of the 25 such frames of these runs was judged by its picture of differences. In the menu of The Witch's House the two sides drew with different fonts (the game asks for `ＭＳ Ｐ明朝` first, which the prefix lacks and vkrgss replaces), so the layout of its text was not compared; The Witch's House 1.09a from its archive with the DLL it ships (2.0.2.1) was compared on its title only (frames 300, 590: text alone differs); the run of Visions & Voices over 11901 frames was repeated and gave the same 13 pictures on each side byte for byte; the name scene was compared (frames 3500, 3630), the load screen and the other screens of the menu were not |
| V2 | VX: walking on a map | Done | The same run: maps with events, weather and messages (frames 1000, 2800, 11300, 11580) equal the original outside the rectangles of text, and five frames of walking without text (11700, 11760, 11900, 11950, 12700) have no differing pixel, after a battle with random numbers. The Witch's House from its archive, a schedule that starts the game and walks: eight frames of its intro and first map (700 to 2150) have no differing pixel, and two frames with its menu (2300, 2400) differ in text alone, where the two sides also chose different fonts | As V1; a map with animated water or a table edge of the game was not looked for among the frames |
| V3 | VX: battle, saving and loading | Done | The same run: two frames of the side-view battle of the game (6600, 7000) equal the original outside the rectangles of text. The Witch's House, a schedule that walks to its save point and saves to file 1 (frames 2055 to 2700 compared, three without text have no differing pixel): the save `Save/1.rvdata` (76577 bytes) and the thumbnail `Save/1.cdat` the game writes beside it are byte for byte those of the original. The same schedule continued: the save is loaded from the game's menu (frames 2700 to 3300; the load screen with the thumbnail differs in text alone, and the five frames without text, three of them after the load, have no differing pixel) | No save of Visions & Voices (its menu has no save command; it saves at events further into the game than the schedule goes); a game name that vkrgss or Ruby 1.8.7 made first keeps a provisional symbol number, which can order a dumped hash another way (not met in this save); in the battle of Visions & Voices the window of a "Missed!" notice is one pixel wider on the original (frame 7000): both sides drew with the game's own font, so the measured width of that text differs by one pixel, which is layout and not glyph pixels; the owner accepted it on 2026-10-12 as a known limit like that of XP, the width of the original being that of GDI under Wine; on the save and load screens of The Witch's House the box around "File 1" follows the width of text drawn with different fonts; as V1 |
| A0 | The original runtime of VX Ace under Wine: a schedule of input, the same captures at the same frame numbers in repeated runs, how waits and fades count as frames | Done | 2026-10-13, Crysalis (a sample game of the publisher, unencrypted) with the DLL it ships (version 3.0.0.1, `RGSS300.dll`), a schedule of 6551 frames through the title, the introduction with its messages, the menu, walking with a transfer between maps, a battle and walking again: 31 window captures and 13 snapshots byte for byte equal in two runs with seed 1 (waits and fades run as loops of `Graphics.update`) and in two runs with seed 20261009 (native waits and fades); where a capture and a snapshot were taken of the same frame at a brightness above 0 they are equal, 13 of 13 in each run with seed 1. The frame rules of `Graphics.update`, `wait`, `fadeout`, `fadein`, `freeze` and `transition` are those of RGSS2: 331 frames of a probe game, 85729 sampled pixels, none wrong, on version 3.0.0.1 and on 3.0.1.1; frame rate and sizes are those of RGSS2 too. In Search of Immortality (version 3.0.1.1, `RGSS301.dll`, 640 by 480) and Capella's Promise 1.09 (version 3.0.1.1 under the name `RGSS300.dll`) start from their archives `Game.rgss3a` with the probe inside: 1501 frames each with the confirm key every 50 frames, 15 captures and 4 snapshots each. Of the interpreter: 2170 symbols exist before the first script and the next serial is 2086; `Marshal` is 4.8; there is no prelude; a method calling itself inside a Fiber reaches depth 284 | A frame inside a transition cannot be held, only the one after it; saving, loading and reset were not measured; the two encrypted games ran once each, without a repeated run; in sequences that call `Input.update` several times without a `Graphics.update`, one of two runs saw a key one update later, so the rule of when a key is seen holds for the frame loop only; no game of the corpus has a name input scene; mask transitions were recorded, not fitted; audio and movies; `String#hash` differs in every process, and Ruby threads run at times that are not repeatable against frames |
| A1 | VX Ace: title screen and menu | Done | 2026-10-14, `cargo xtask oracle` on `main` at `51c8d34`, seed of the harness 20261009; Wine 11.0 in the sandbox of the harness (no GPU device node, no network), Xvfb, llvmpipe on the side of vkrgss; the built-in scripts extracted from each game's own DLL. Crysalis (unencrypted) with the DLL it ships (`RGSS300.dll`, version 3.0.0.1, `RTP=RPGVXAce`), its schedule through the title, New Game, the introduction confirmed, the map, the menu opened, the cursor moved and the menu closed, walking into a cave, a battle with one Ratinga fought to its end and back on the map, 28 frames compared: the title and its menu (frames 60, 210, 360) equal the original pixel for pixel, text included; the menu (4950, 5000) differs in 3 pixels in each frame, the antialiased edge of the glyph "Q" of "Quit" (largest difference 5 of 255), which is a glyph shape. In Search of Immortality from its archive `Game.rgss3a` with the DLL it ships (`RGSS301.dll`, version 3.0.1.1, `resize_screen` to 640 by 480), a schedule of C every 16 frames until control returns, then the menu, Save, File 1, Game End, To Title, Continue, File 1, 15 frames compared: its account screen and title (140, 180), the save screen (4700, 4759), the menu after the save (4800), the title with Continue (5400, 5559) and the load screen (5600, 5659) equal the original pixel for pixel, text included. The original's pictures of both games equal those of an earlier run of the same schedule byte for byte | Frames inside a transition cannot be compared (Crysalis 4900 and 5450), as for VX; the third game of the corpus, Capella's Promise, was started and captured on the original's side only; audio is not compared; the change `match-the-endings-and-loading-of-rgss3` (endings, error reports, `require`) is not part of 0.3.0 |
| A2 | VX Ace: walking on a map | Done | The runs of A1 (Crysalis with `RGSS300.dll` and In Search of Immortality with `RGSS301.dll`, seed 20261009, the same schedules, the original's side repeated): Crysalis on the map after the introduction (4850), walking into the cave (5050 to 5400) and back on the map after the battle (6200, 6250); In Search of Immortality on its map (4200, 4499) and after the save (4950): every one of these frames equals the original pixel for pixel, text included | In Search of Immortality is never walked by its schedule, so it meets no random encounter; as A1 |
| A3 | VX Ace: battle, saving and loading | Done | The runs of A1 (Crysalis with `RGSS300.dll` and In Search of Immortality with `RGSS301.dll`, seed 20261009, the same schedules, the original's side repeated): the battle of Crysalis and its log (5500 to 6100) equal the original pixel for pixel, text included. In Search of Immortality saves through its own menu at frame 4760: `Save01.rvdata2` (19018 bytes) and the game's own `AutoSave.rvdata2` (21992 bytes, written at its map transfers) are byte for byte those of the original; it returns to the title and loads file 1, and the map after the load (5760, 5999) equals the original pixel for pixel. Both saves of the original equal those of an earlier run byte for byte | F12 and the reset are covered by tests of vkrgss alone; a save whose music is an Ogg file holds `Audio.bgm_pos`, which vkrgss computes by the measured model and in which the original's own runs can differ by whole chunks (the save compared here plays an MP3, whose position is 0 on both sides); as A1 |
| P1 | ARM Linux (aarch64): frames and saves equal those of x86_64 byte for byte for the same games, frames and input (see "Platforms") | Done | 2026-10-16, the owner's Jetson Orin Nano (`duck-stable`, Ubuntu 24.04.5, gcc 13.3, llvmpipe) against the development machine (x86_64), both at `3259c0d4`, `cargo xtask oracle --vkrgss-only` with the frames and input schedules of the comparisons with the original: the seven XP games, Visions & Voices, Crysalis and In Search of Immortality, 84 frames, all identical pixel for pixel; `Save01.rvdata2` and `AutoSave.rvdata2` of In Search of Immortality identical byte for byte (and equal to the original's on x86_64 the same day). Both interpreters built with verified headers of aarch64 (change `build-the-interpreters-for-aarch64`); the whole gate, the stressed suites included, passes on the Jetson at `da8088a7` after the collector guards of change `guard-values-held-by-inner-pointers`, which the stressed suite of Ruby 1.9.2 on aarch64 found | The comparison ran on llvmpipe, not on NVIDIA's driver of the Jetson (the Vulkan tests pass on both); audio and a windowed run were not checked on aarch64; glyphs are those of the same font file on both sides |
| P2 | macOS (Apple silicon, MoltenVK): the games of P1 with the same frames and saves as on x86_64, tested by hand on the owner's MacBook | Done | 2026-10-18, the owner's MacBook Air (Apple M4, MoltenVK) against the development machine (x86_64), both after change `make-the-platform-runs-comparable`, `cargo xtask xarch` with the games of P1: the seven XP games, Visions & Voices, Crysalis and In Search of Immortality, 84 frames identical pixel for pixel and 17 saves identical byte for byte. The app bundle (changes `package-a-macos-app`, `bundle-a-loader-for-macos-14`, 2026-10-20) carries its own Vulkan loader and MoltenVK and needs macOS 14; the owner opened it at the Mac on 2026-10-21 and its window works 2026-10-23, the owner at the MacBook Air (macOS 27.0.1) with the app built from `main` at `f853e38e`: Crysalis (VX Ace), Visions & Voices (VX) and 群战示范工程 (XP, Chinese text) each in a window, with the guest in a process; the owner reported the window, the games and the audio working. | Audio was checked by ear, not compared with the original; a guest process is not confined on macOS; the app is signed ad hoc and not notarized; the comparison composited on the Apple GPU, the Mac having no CPU rasteriser |
| P3 | Android (aarch64): a measurement of feasibility first; the acceptance is fixed after it | In progress | The measurement of 2026-10-16/17 on both test phones fixed the minimum at API level 34 (see "Android"). The app imports a game as a zip, has on-screen buttons and gamepads, and runs the guest in a process under seccomp with save states. 2026-10-18, `cargo xtask xarch android` on the Pixel 7a and the Galaxy S24+ against x86_64 with the games of P2: 84 frames identical pixel for pixel and 17 saves identical byte for byte on each phone. The release workflow builds an APK signed with the release key (change `prepare-the-android-release`, 2026-10-20) | The acceptance itself is not yet written down; audio output; most Android kernels have no Landlock; the APK is self-signed and not on Google Play |
| P4 | Windows (x86_64), without save states; the acceptance is fixed when the work starts | Not started | | |

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

Risks of the extension to VX and VX Ace, each with what answers it (change
`extend-scope-to-vx-and-vx-ace`):

- The original runtime of VX does not run under Wine, or its frames cannot be captured
  exactly. V0 comes first and alone; without it the standard of "Goal and scope" cannot be
  checked.
- No VX game by the owner exists and free games are few. The list is made and shown before
  V0; a thin corpus is said in the milestone table, as for XP.
- Rules measured for XP are assumed for VX. Each is measured again on `RGSS202E.dll` before
  it is relied on; the probes exist.
- A change to shared arithmetic or to the interpreter breaks XP. A change that touches shared
  code names the probes and games of XP it was checked against; the seven-game comparison is
  run at its merge.
- At 60 frames per second a frame has 16.7 ms, and waves, blur and snapshots are new costs.
  Frame times of representative VX scenes are measured before 0.2.0.

  Measured on 2026-10-13 (change `release-0-2-0`): a release build of `main` at `0bf50cf`,
  run with `--headless --no-audio --frame-stats`, which neither waits between frames nor
  presents; AMD Ryzen 5 9600X, its integrated GPU with RADV (Mesa 26.2.4), frames of 544 by
  416. The stages are those `docs/running.md` describes under "Frame times"; times are in
  milliseconds.

  | Run | Frames | `guest` mean | `guest` largest | `submit` mean | `submit` largest | `compose (device)` mean | `compose (device)` largest |
  |---|---|---|---|---|---|---|---|
  | Visions & Voices: title, name input, maps with messages and weather, a battle, walking, the menu over its blurred snapshot | 13500 | 0.208 | 39.611 | 0.742 | 2.937 | 0.689 | 1.890 |
  | The Witch's House: title, opening, walking on its first map | 3200 | 0.394 | 32.654 | 0.825 | 2.814 | 0.751 | 1.738 |

  `encode`, `bin` and `upload` together have a mean below 0.01 ms in both runs and a largest
  single value of 1.215 ms (`upload`). Of the guest's own rows, which are written per 600
  frames:

  - `guest scripts`: the highest mean of a block is 0.543 ms (Visions & Voices, the battle)
    and 0.409 ms (The Witch's House, walking). The largest single frames are 37.8 ms,
    28.6 ms and 26.4 ms in Visions & Voices and 28.4 ms in The Witch's House, each in a
    block in which a scene is entered (the name input, the menu and the return from it, the
    start of the game). The statistics give the largest value of a block and not the frame
    it belongs to, so that these are the frames of the scene changes is an inference.
  - `guest bitmap operations`: largest 11.6 ms in the block in which the battle begins, and
    2.2 ms in the blocks in which the menu takes and blurs its snapshot; the mean of every
    block is below 0.01 ms.
  - `guest text`: largest 8.0 ms; the mean of every block is below 0.02 ms.
  - `guest extraction`: largest 1.9 ms; the mean of every block is below 0.02 ms.

  A frame costs about 1 ms in the mean on this machine, a sixteenth of the 16.7 ms it has.
  Single frames in which a scene is built take more than one frame's time on the game's
  side, up to 40 ms. Not measured: how long the original takes for the same frames, a slower
  processor, and a run with a window. No performance requirement is made of these numbers.
- Ruby 1.9.2 differs in encodings, in what it serialises and in how it is embedded. These
  are checked for feasibility when VX Ace is planned; sharing the rest of vkrgss does not
  settle them. They were checked on 2026-10-13: the release builds as a static library and
  embeds, with the risks below.

Risks of VX Ace, named by the study of 2026-10-13, each with what answers it (change
`plan-vx-ace`):

- Ruby 1.9.2-p0 does not work with a current compiler as it is: Fibers needed fixes, and
  two values were lost to the conservative collector. More lost values must be expected.
  The stress run of the collector is part of the gate from the start.
- Ruby threads are native threads: a copy made by `fork` has none of the other threads, and
  when a thread runs is not repeatable. A save state is refused while a script thread is
  alive, and such a run is not claimed to be reproducible ("VX Ace").
- Strings have encodings throughout the binding: every string it creates has to be
  labelled, and the original knows fewer encodings than the release. This is work, not a
  doubt; what the original has is measured first.
- Glyph shapes differ, as before; text is compared by layout and advance widths.
- `DL` lets a script reach into the memory of the runtime. It gets a decision of its own
  when a game needs it ("Corpus").
- `pow` of glibc differs from the C runtime of the DLL in the last bit for `10.0**23`. The
  rest of `Math` is not surveyed.

Risks of the platforms (change `plan-the-platforms-and-modernization`):

- CORE-MATH may not cover every function of Ruby's `Math`; its coverage is checked before
  it is vendored, and a gap is decided then.
- Flatpak's own seccomp filter may refuse the guest's Landlock or seccomp setup; then Linux
  gets AUR and the tarball only.
- Android may not carry both interpreters on bionic, a separate guest under seccomp, or
  Vulkan 1.3 on the test phones; the measurement of feasibility comes first and alone.
- MoltenVK may lack something the compositor uses (the number of bitmap slots, 64-bit
  integer emulation in the shaders); found by the first runs on the MacBook.

Open, to be decided before the change that implements it is drafted: how
`Graphics.snap_to_bitmap` of VX gets the pixels of the scene as it is, also while the display
is frozen, without a frame going by. Composition happens on the host today and no message
brings pixels back to the guest. The decision keeps the plain-data boundary and the purity of
the core, and names the messages it adds or its use of the reference compositor.
