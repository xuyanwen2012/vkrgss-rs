# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-host`: `run_windowed(WindowedOptions, guest)` runs `guest: FnOnce(GuestEnd)` on a thread
  it spawns and the session in a window on the calling (main) thread, and returns a
  `SessionEnd`; closing the window sends the guest a quit request and waits for its shutdown
  message. `headless_link()` returns a `GuestEnd` and a `HostLink`, and
  `run_headless(HeadlessOptions { device, frames, script, dumps, errors }, link)` returns a
  `HeadlessReport` or a `HeadlessError`. `device_from_text` and `InputScript::parse` read the
  command-line forms. The host drops audio commands with one notice.
- `vkxp-binding`: `run_game(ruby, config, link)`, to be called on the interpreter thread, with
  a `Config` holding the game's files, the configuration read from `Game.ini`, the built-in
  scripts file, the default font names, the debug flags, the process-function switch and the
  text backend (`Box<dyn TextBackend + Send>`: core's `TextRasterizer` plus `font_exists`).
- `vkxp-ruby-sys`: `Interpreter::start` creates the interpreter thread; `Interpreter::run`
  sends a closure there and waits. Its documentation asks every other thread to block
  SIGVTALRM.
- `vkxp-text`: `TextEngine::new(&FontSources)` with `measure` and `rasterize` (both `&mut
  self`, taking a `TextRequest { families, size, bold, italic }`) and `is_available`.
  `FontSources` holds system fonts, caller directories (earlier ones first), substitutions and
  fallback families. Its two test fonts are in `crates/vkxp-text/tests/fonts/`.
- `vkxp-vfs`: `GameIni::read(dir, encoding)`, `resolve_rtps`, `GameFiles::new`.
- The dependency rules let only the `vkxp` binary depend on both guest and host crates.
- Two of the original DLLs supplied with the RTP (versions 1.00 and 1.02) store their data
  section uncompressed, with the built-in scripts as two long NUL-terminated strings that both
  start with `module RPG;`; an unpacked later DLL has the same strings. Later DLLs as shipped
  are compressed and show nothing.
- Vulkan tests refuse to run unless the loader is restricted to chosen drivers.

## Goals / Non-Goals

**Goals:**

- `vkxp <dir>` plays a game; `vkxp <dir> --headless ...` is the tool every later change uses
  to check a real game.
- No Enterbrain source in the repository.

**Non-Goals:**

- A configuration file; everything is a command-line option for now.
- Audio output.

## Decisions

### Process layout

`main`:

1. Parse arguments with a small hand-written parser (the option set is tiny).
2. `GameIni::read`, resolve the RTP names against the `--rtp` table, build `GameFiles`, find the
   built-in scripts file. Every failure up to here is a message and a non-zero exit, before any
   window or interpreter exists.
3. Make the game directory the working directory.
4. Block SIGVTALRM in the main thread before any other thread exists; the closure that runs the
   game unblocks it on the interpreter thread.
5. Build the text backend and the binding's `Config`; start the `Interpreter`.
6. Run the host on the main thread.
   - Windowed: `run_windowed(options, guest)`, where `guest` receives the `GuestEnd` on the
     thread the host spawns and calls `interpreter.run(|ruby| run_game(ruby, config, end))`,
     keeping the result.
   - Headless: `headless_link()`, a thread that does the same with the `GuestEnd`, and
     `run_headless(options, link)` on the main thread.
7. Join the guest, drop the interpreter and exit with the combined status: non-zero if the
   game reported a failure or the host returned an error or an unsuccessful end.

A guest that does not answer a quit request would keep the process alive, since a script can
loop without ever calling `Graphics.update`. After the host has asked the guest to quit (window
closed, or the frame limit of a headless run), the program waits a few seconds for the guest
and then exits with a message and a non-zero status.

### Options

```text
vkxp [GAME_DIR]
  --rtp NAME=DIR            repeatable
  --builtin-scripts FILE    default: $XDG_DATA_HOME/vkxp/rpg_builtin.rb
  --device INDEX|NAME
  --font-dir DIR            repeatable
  --encoding utf-8|gbk|shift-jis|big5
  --debug
  --allow-process-functions
  --headless  --frames N  --input SCRIPT|@FILE  --dump FRAME=FILE (repeatable)
```

### Text backend

`FontBackend` in the binary crate wraps a `TextEngine` in a `RefCell` (the trait takes `&self`,
the engine `&mut self`, and everything runs on the interpreter thread) and implements the
binding's `TextBackend`: a core `Font` becomes a `TextRequest` (names in order, size, bold,
italic; a size below 1 is drawn as 1), the engine's mask becomes core's `Mask`, and
`font_exists` is `is_available`. Its sources are the `--font-dir` directories, then the game's
`Fonts` directory if present, then the system's fonts. It lives here rather than in the binding
so that the binding does not depend on the font engine.

### Extraction tool

`xtask extract-rpg` scans the file for NUL-terminated byte strings that start with
`module RPG;` and consist of printable text, and writes them in file order separated by line
breaks. It reads any file, so it works on the old DLLs and on an unpacked one. It lives in
`xtask` because it is a developer tool, not part of the runtime.

### End-to-end test

A test of the `vkxp` crate assembles a game in a temporary directory with the binding's test
support (a `Game.ini`, a script archive with a few scripts that create a bitmap, a sprite and
a window, draw text, move with input and exit), a stand-in built-in scripts file, and runs the
built executable (`CARGO_BIN_EXE_vkxp`) headless on llvmpipe with dumps, an input script and
`--font-dir` pointing at the test fonts of `vkxp-text`. It asserts the exit status, pixels of
the dumps and that two runs agree. Like the other Vulkan tests it refuses to run while the
loader is not restricted, and passes the restriction on to the child process.

## Risks / Trade-offs

- [A user runs without extracting the built-in scripts] → The error names the tool and the
  README explains it.
- [Default device selection picks a GPU the user wants left alone] → `--device` and the loader
  restriction; the README states what to pass on the development machine.
- [The interpreter is started on the main thread's behalf but used from the host's guest
  thread] → `Interpreter::run` is what crosses threads; the extra thread only waits.

## Follow-ups

- The wait for a guest that does not answer a quit request is implemented for headless runs
  only. In a window the program cannot tell when the quit request went out: `run_windowed`
  keeps the window until the guest's shutdown message and joins the guest thread itself before
  it returns, so a script that loops without calling `Graphics.update` keeps the window, and
  after a host error the process, alive. `vkxp-host` would have to bound that wait itself or
  report the close request to its caller.
- The end-to-end test cannot make a font family exist only in a `--font-dir` directory: the
  system's fonts are always a source, and the test fonts of `vkxp-text` may be installed on
  the machine. The test relies on a font directory coming before the system's fonts. A way to
  run without system fonts would make it independent of the machine.
- The default font name is `Arial`, or `ＭＳ Ｐゴシック` when the name of the game's library
  ends in `J`. Whether the original runtime's Japanese edition starts with exactly that name
  has not been measured.
