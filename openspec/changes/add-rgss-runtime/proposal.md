# Proposal

## Why

The binding has values and bitmaps, but a game is more than objects: it is a list of scripts
that must be loaded and run, and those scripts drive everything through three modules,
`Graphics`, `Input` and `Audio`, and a few global functions. Until those exist nothing runs.

## What Changes

- Running a game on the guest side: take the game's configuration (read from `Game.ini` by the
  caller) and files, prepare the interpreter as the original runtime does (`$KCODE`, empty load path, `$DEBUG`, `$BTEST`), load the built-in `RPG` scripts
  from the locally extracted file, read the script archive (`Scripts.rxdata`), evaluate each
  script in order under its section name, report an uncaught error with script name and line,
  and tell the host when the game ends.
- `Graphics`: `update` (extract a frame from the scene, send it, wait for the tick, react to
  reset and quit), `freeze`, `transition`, `frame_reset`, `frame_rate`, `frame_count`.
- `Input`: `update`, `press?`, `trigger?`, `repeat?`, `dir4`, `dir8` and the button constants,
  on top of core's input state and the host's snapshots.
- `Audio`: the eleven functions, which find and read the sound file and send the host an audio
  command.
- Global functions and classes: `load_data`, `save_data`, `print`, `p`, the exception classes
  `Reset` and `Hangup`, a placeholder `Win32API`, and the default-off process functions
  (`system`, backticks, `exec`, `fork`, `syscall`, `IO.popen`) with `require` and `load` limited
  to Ruby files inside the game directory.
- Reset on request of the host (F12): the scripts start over from the first.

Out of scope: the scene classes (`Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap`), the
executable, a drawn message panel for `print` and `p` (the text goes to the error stream and the
host), emulation of specific `Win32API` calls.

## Capabilities

### New Capabilities

- `rgss-runtime`: how a game's scripts are loaded, run, reset and ended, and the global
  functions and policies scripts meet.
- `rgss-modules`: the Ruby modules `Graphics`, `Input` and `Audio`.

### Modified Capabilities

None.

## Impact

- Crate `vkxp-binding`: new modules (`runtime`, `graphics`, `input`, `audio`, `kernel`) and
  tests; it now uses `vkxp_proto::link` and core's scene and input state.
- The built-in `RPG` scripts are read from a file outside the repository; tests use a small
  stand-in written for the purpose.
- The executable change builds directly on the entry point added here.
