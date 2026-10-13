# vkxp design decisions

Record of the decisions settled on 2026-10-08 before any code was written. Treat these as
settled; change one only through an OpenSpec change that says so.

The change `extend-scope-to-vx-and-vx-ace` changed the scope on 2026-10-12, by decisions of
the owner taken that day: from RPG Maker XP alone to RGSS1, RGSS2 and RGSS3. It rewrote "Goal
and scope" and added "Several runtimes" and the rows V0 to V3 of "Milestones".

## Goal and scope

- vkxp is a from-scratch runtime for the games of RPG Maker XP, VX and VX Ace, built to a
  2026 standard.
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
- Linux is the primary platform. Android is deferred. macOS is a distant maybe.
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
    has passed. Comparing with vkxp begins at V1.
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

- VX Ace: after VX; an own fork of Ruby 1.9.2 is vendored and patched to the behaviour
  measured on the original, as was done with Ruby 1.8.7; its guest is an executable of its
  own, which keeps the two interpreters' symbols and global state apart. How it is
  packaged and chosen, what `--guest thread` means for it, and everything else about
  VX Ace is decided after V3.

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
  kept outside the repository and only a copy of it is run: by vkxp in the confined guest
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

### Process

- As before: OpenSpec changes implemented by flows, at most three in parallel. The owner's
  standing go-ahead covers drafting, launching, merging and archiving up to V3 and does not
  cover VX Ace. Pushing to the public repository is asked for each time.

## Milestones

Status as of 2026-10-09. "Evidence" names what was checked; "Not covered" is what the status
does not claim.

The owner confirmed on 2026-10-09 that all milestones M0 to M6 are complete as recorded here,
accepting the limits listed under "Not covered". Work beyond them is new work. That
confirmation concerns M0 to M6 only. The rows V0 to V3 are the milestones of VX, decided on
2026-10-12 (see "Order and milestones"); a row is set to "Done" only with the record that
section asks for. V0 is a proof of the original's side alone and was done on 2026-10-12.

| | Outcome | Status | Evidence | Not covered |
|---|---|---|---|---|
| M0 | Workspace skeleton, guest/host protocol, oracle harness | Done | `cargo xtask oracle` compares a game frame by frame with the original runtime under Wine; used on eight game set-ups | |
| M1 | Title screen of the test game, navigable | Done | The test game has no title screen and goes straight to its map; its menu, and the title screens of two other games, are navigable and compared with the original | |
| M2 | Map: tilemap, viewports, planes, walking | Done | Map frames of the test game, a second large game and three more games are identical to the original pixel for pixel; tilemaps are evaluated by the compositor as decided | |
| M3 | Audio: Ogg, WAV, MP3, MIDI | Done | Decoding and mixing tests; the owner confirmed sound in a windowed run of a real game | What the original does with more than one MIDI loop marker; WMA without `ffmpeg` installed |
| M4 | Full play-through: battle, menus, save/load, F12 reset | Done | Battle course, menu and a save written at the same frame (byte for byte) equal the original; reset is covered by tests of vkxp alone | A complete play-through of a long game by a person; F12 cannot be injected into the original for comparison |
| M5 | Modernisation: filters, present-timing pacing, configuration, gamepad | Done | Scaling filters, fullscreen, layered TOML configuration, gamepads, input recording and replay; `VK_EXT_present_timing` pacing runs on the AMD GPU at 60, 120 and 144 Hz with a mean interval of 25.0 ms | Whether present-timing pacing looks smoother than timer pacing on a physical display has not been judged by a person |
| M6 | Wider compatibility: encrypted archives, `Win32API` shims, more games | Done | A packed game is identical to the original on every compared frame with both runtimes reading only the archive; common `Win32API` calls and raw key state are emulated; 20 of 23 further XP games start and the three others stop on files missing from the projects | An archive written by the original packing tool; custom DLLs that games ship |
| V0 | The original runtime of VX under Wine: a schedule of input, the same captures at the same frame numbers in repeated runs, how waits and fades count as frames | Done | 2026-10-12, Visions & Voices (its DLL is version 2.0.0.1 under the name `RGSS202E.dll`), seed 1, a schedule of 11901 frames through title, name input, a battle, walking and the menu: 17 window captures and 9 snapshots byte for byte equal in two runs, and again with seed 20261009. The frame rules of `Graphics.update`, `wait`, `fadeout`, `fadein`, `freeze` and `transition` hold on 331 frames of a probe game with no wrong pixel, on that DLL and on version 2.0.2.1. The Witch's House starts from its archive with the probe inside | A Ruby thread ends the original under Wine, so the game was run with thread blocks executed at once; a frame inside a transition cannot be held, only the one after it; mask transitions were recorded, not fitted; audio; the third game of the corpus is not unpacked |
| V1 | VX: title screen and menu | In progress | 2026-10-12, Visions & Voices with the DLL it ships (2.0.0.1), `cargo xtask oracle` with `--no-threads`, seed of the harness, the input schedule of V0 extended by the keys that open the menu: the title (frames 100 and 261, the second inside its fade-out) and the menu over its blurred snapshot (frames 12100 to 12550) equal the original outside the rectangles of text; frame 261 has no differing pixel | The pixels of text are not left out by the harness yet, so every frame with text counts as different and was judged by its picture of differences; The Witch's House 1.09a from its archive with the DLL it ships (2.0.2.1) was compared on its title only (frames 300, 590: text alone differs); the run of Visions & Voices over 11901 frames was repeated and gave the same 13 pictures on each side byte for byte; the name scene was compared (frames 3500, 3630), the load screen and the other screens of the menu were not |
| V2 | VX: walking on a map | In progress | The same run: maps with events, weather and messages (frames 1000, 2800, 11300, 11580) equal the original outside the rectangles of text, and five frames of walking without text (11700, 11760, 11900, 11950, 12700) have no differing pixel, after a battle with random numbers. The Witch's House from its archive, a schedule that starts the game and walks: eight frames of its intro and first map (700 to 2150) have no differing pixel, and two frames with its menu (2300, 2400) differ in text alone, where the two sides also chose different fonts | As V1; a map with animated water or a table edge of the game was not looked for among the frames |
| V3 | VX: battle, saving and loading | In progress | The same run: two frames of the side-view battle of the game (6600, 7000) equal the original outside the rectangles of text. The Witch's House, a schedule that walks to its save point and saves to file 1 (frames 2055 to 2700 compared, three without text have no differing pixel): the save `Save/1.rvdata` (76577 bytes) and the thumbnail `Save/1.cdat` the game writes beside it are byte for byte those of the original | No save was loaded on either side; no save of Visions & Voices; a game name that vkxp or Ruby 1.8.7 made first keeps a provisional symbol number, which can order a dumped hash another way (not met in this save); the differences of text have not been put to the owner; as V1 |

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
- Ruby 1.9.2 differs in encodings, in what it serialises and in how it is embedded. These
  are checked for feasibility when VX Ace is planned; sharing the rest of vkxp does not
  settle them.

Open, to be decided before the change that implements it is drafted: how
`Graphics.snap_to_bitmap` of VX gets the pixels of the scene as it is, also while the display
is frozen, without a frame going by. Composition happens on the host today and no message
brings pixels back to the guest. The decision keeps the plain-data boundary and the purity of
the core, and names the messages it adds or its use of the reference compositor.
