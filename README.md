# vkxp

A Rust + Vulkan runtime for RPG Maker XP (RGSS1) games. Linux first, MIT licensed.

The project can start a game: `vkxp <game directory>` runs the game's scripts in the embedded
Ruby interpreter against the RGSS classes and shows the frames of the Vulkan compositor in a
window, or writes them to files in a headless run. In a window it plays the game's audio:
Ogg Vorbis, WAV, MP3 and, with a SoundFont, MIDI. A game's encrypted archive (`Game.rgssad`)
is read. Settings come from configuration files and the command line, and gamepads work
besides the keyboard.
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
host keys are F12 (reset), Alt+Enter or F11 (borderless fullscreen), F2 (frame rate in the
title) and Tab held (fast-forward). Closing the window asks the guest to quit.

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
| `--config FILE` | The user's configuration file, instead of the default location. See *Configuration files*. |
| `--no-config` | Read no configuration file. Not together with `--config`. |
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`. Repeatable. A game that names a package without this option does not start. |
| `--builtin-scripts FILE` | The built-in scripts file, instead of the default location. |
| `--device INDEX\|NAME` | The Vulkan device: its index or a part of its name, as `vulkaninfo --summary` prints them. In a window the default is the first discrete GPU, then an integrated one; headless it is `llvmpipe`. |
| `--font-dir DIR` | A directory of fonts. Repeatable; earlier directories come first, then the game's `Fonts` directory, then the fonts installed on the system. |
| `--font-own-metrics` | Size every font by its own line box. Without it a font that replaces a Windows font is scaled to the line box of the font it replaces. Meant for comparing text with the original when both draw with the same font file. |
| `--encoding ENCODING` | The text encoding of `Game.ini`: `utf-8`, `gbk`, `shift-jis` or `big5`. Without it UTF-8 is tried, then those three in that order. |
| `--filter FILTER` | How the frame is scaled into the window: `auto`, `nearest`, `linear` or `sharp`. See *Window*. |
| `--fullscreen` | Start in borderless fullscreen. |
| `--debug` | Debug mode: `$DEBUG` is `true`. |
| `--allow-process-functions` | Lets `system`, backticks, `exec`, `fork`, `syscall` and `IO.popen` start processes. Without it they do nothing. |
| `--log-p` | `p` only writes to the error stream: no message panel is shown and the game does not wait. See *Messages*. |
| `--no-audio` | Run without sound. No audio device is opened. |
| `--soundfont FILE` | The SoundFont that MIDI music is played with. A file that does not exist ends the program with an error before any window opens. |
| `--seed N` | The seed of the random number generator, set before any script runs and again on every reset. Without it a headless run uses 0, a replay the seed of its recording, and a windowed run one taken from the clock. |
| `--preload FILE` | A Ruby file evaluated after the built-in scripts and before the game's first script, and again after a reset. Repeatable, in the order given. |
| `--record FILE` | Write the input of every frame and the seed to `FILE`. |
| `--replay FILE` | Play the input of a recording back instead of the keyboard. Not together with `--record` or `--input`. |
| `--headless` | Run without a window, on a virtual clock that never waits. |
| `--frames N` | Headless: stop after N frames if the game has not ended. |
| `--input SCRIPT\|@FILE` | Headless: the buttons held from given frames on, in the syntax described under *Running the demo*, or a file that holds such a script. |
| `--dump FRAME=FILE` | Headless: write frame `FRAME`, counted from 0, as a PNG file. Repeatable. |

An unknown option or a malformed value, a malformed input script file included, ends the
program with the usage message and status 2.
The exit status is 0 when the game ends normally: its scripts return or call `exit`, the
window is closed, or the frame limit of a headless run is reached. It is non-zero when a
script error ends the game (its report is on the error stream), when the host fails, when a
frame is rejected in a headless run, or when a frame that was to be dumped was never reached.

### Configuration files

What the options above set can be written down in two files, both optional: the user's, at
`$XDG_CONFIG_HOME/vkxp/config.toml` (`~/.config/vkxp/config.toml` unless `XDG_CONFIG_HOME` is
set) or wherever `--config` says, and `vkxp.toml` in the game directory. The game's file
overrides the user's and the command line overrides both: a value replaces the value, a list
replaces the whole list, and the tables `[rtp]`, `[keys]` and `[gamepad]` are overridden entry
by entry. `--no-config` reads neither file. Paths in a file are relative to the directory of
that file. A file that does not parse, or that holds an unknown key, a value of the wrong type
or an unknown name of a key or button, ends the program with an error that names the file and
what is wrong in it, before any window opens.

Whether settings go together is decided once the files and the command line are put
together, with the same rules and messages as for the options: `frames`, `input` and `dump`
need a headless run, whether `headless = true` or `--headless` makes it one, and `replay`
goes with neither `record` nor `input`, wherever each of them is set.

This example holds every setting; `replay` is a comment in it because it does not go together
with `record` and `input`. Its `[keys]` and `[gamepad]` tables are the default maps, so
leaving them out changes nothing:

```toml
builtin_scripts = "rpg_builtin.rb"           # --builtin-scripts
device = "RADV"                              # --device; an index is written "0"
font_dirs = ["fonts"]                        # --font-dir, in this order
font_own_metrics = false                     # true is --font-own-metrics
encoding = "gbk"                             # --encoding
debug = false                                # true is --debug
allow_process_functions = false              # true is --allow-process-functions
log_p = false                                # true is --log-p
filter = "auto"                              # --filter
fullscreen = false                           # true is --fullscreen
audio = true                                 # false is --no-audio
soundfont = "/usr/share/soundfonts/FluidR3_GM.sf2" # --soundfont
seed = 7                                     # --seed
preload = ["patches/fonts.rb"]               # --preload, in this order
headless = true                              # true is --headless
frames = 600                                 # --frames
input = "40:C, 42:"                          # --input; "@keys.txt" names a file
dump = ["100=frame.png"]                     # --dump FRAME=FILE, one entry per frame
record = "session.txt"                       # --record
# replay = "session.txt"                     # --replay; not together with record or input

[rtp]                                        # --rtp NAME=DIR
Standard = "/home/me/rtp/Standard"

[keys]
DOWN = ["ArrowDown", "Numpad2"]
LEFT = ["ArrowLeft", "Numpad4"]
RIGHT = ["ArrowRight", "Numpad6"]
UP = ["ArrowUp", "Numpad8"]
A = ["ShiftLeft", "ShiftRight", "KeyZ"]
B = ["Escape", "Numpad0", "Insert", "KeyX"]
C = ["Space", "Enter", "NumpadEnter", "KeyC"]
X = ["KeyA"]
Y = ["KeyS"]
Z = ["KeyD"]
L = ["KeyQ", "PageUp"]
R = ["KeyW", "PageDown"]
SHIFT = ["ShiftLeft", "ShiftRight"]
CTRL = ["ControlLeft", "ControlRight"]
ALT = ["AltLeft", "AltRight"]
F5 = ["F5"]
F6 = ["F6"]
F7 = ["F7"]
F8 = ["F8"]
F9 = ["F9"]

[gamepad]
A = ["West"]
B = ["East", "Select"]
C = ["South", "Start"]
X = ["North"]
L = ["LeftShoulder"]
R = ["RightShoulder"]
```

In `[keys]` and `[gamepad]` a button, by the name of its `Input` constant, is held by exactly
the keys or pad buttons listed for it; an empty list leaves it without any. Buttons that are
not mentioned keep their defaults. The name of a button may be written in either case, and
`c` in one file is the same entry as `C` in another; a file that lists one button twice is
refused. A key is named as the key codes of `winit` are
(`winit::keyboard::KeyCode`): `KeyA` to `KeyZ`, `Digit0` to `Digit9`, `Numpad0` to `Numpad9`,
`F1` to `F12`, `ArrowUp`, `Space`, `Enter`, `Escape`, `ShiftLeft`, `ControlRight` and so on.
They name the position of a key, not the letter a keyboard layout puts on it. The host keys
(F12, F2, F11, Tab, Alt+Enter) are not part of the map.

### Gamepads

A windowed run reads every connected gamepad together with the keyboard: a button is held
while a key or a pad holds it. The directional pad and the left stick, pushed more than half
of its way along an axis, are the directions. The other pad buttons hold what `[gamepad]`
says, by default as in the example above; their names are `South`, `East`, `West`, `North`
(the four face buttons by position), `LeftShoulder`, `RightShoulder`, `LeftTrigger`,
`RightTrigger`, `Select`, `Start`, `LeftStick` and `RightStick` (the sticks pressed). A pad
may be connected or removed while the game runs, and a removed pad releases what it held. On
a machine where gamepads cannot be read this is said once on the error stream and the game
runs with the keyboard. A headless run reads no gamepad.

### In a window

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --device RADV
```

The loader restriction is part of the command. `--device` only chooses among the devices the
loader enumerates, and without it the program chooses a GPU by itself; either way the loader
loads the driver of every GPU in the machine unless it is restricted. On the development
machine the Intel Arc B580 must not be opened, so a windowed run there is the command above,
on the AMD integrated GPU, or the same on the CPU with `VK_LOADER_DRIVERS_SELECT='*lvp*'` and
`--device llvmpipe`.

The host keys are those of the demo: F12 (reset), Alt+Enter or F11 (borderless fullscreen),
F2 (frame rate in the title) and Tab held (fast-forward). Closing the window asks the game to
quit.

### Messages

`print`, `p` and the report of an uncaught script error are written to the error stream and
shown in a panel over the picture: white text in a dark box in the middle of the screen,
wrapped to the box and cut with `...` when it has more lines than fit. The game waits until
the message is dismissed with a key that holds C or B (by default Space, Enter, C, Escape, X
and so on) or a mouse click. That press does not reach the game. A headless run dismisses
every message at once. With `--log-p`, `p` writes to the error stream only and the game goes
on. While the screen is frozen (`Graphics.freeze`) the panel is not visible; the message is
still on the error stream and still waits for its dismissal.

### Window

The window opens with the game's screen at the largest whole-number scale that fits the
monitor, and can be resized freely; the picture keeps its shape, with black bars where the
window has another. Alt+Enter and F11 switch to borderless fullscreen on the monitor the
window is on and back to the earlier window size, and `--fullscreen` starts that way. Neither
key reaches the game as a button while it toggles.

`--filter` chooses how the picture is scaled. `nearest` shows for every pixel of the window
the game pixel under its centre, which is exact at whole-number scales and uneven at others.
`linear` interpolates between the four nearest game pixels and is soft. `sharp` (sharp
bilinear) is nearest except in a band one window pixel wide along the borders between game
pixels, where it interpolates. `auto`, the default, is `nearest` when the scale is a whole
number and `sharp` otherwise. A headless run has no window and ignores both options.

Frames are shown at the game's frame rate, 40 per second unless the game changes it. Where
the Vulkan device and the window system support `VK_EXT_present_timing`, every frame is given
the time it is to appear on screen, one frame period after the frame before it, so that on a
display whose refresh rate is no multiple of the frame rate each frame stays up for as even a
time as the display allows; with `VK_KHR_present_wait2` as well, a frame is only handed over
once the one before it has reached the display. Without the extension, and after any error
from it, which is reported once, frames are shown at the first refresh after they are ready.
`--present-stats` writes the intervals between the frames shown to the error stream, every
600 frames and at exit, as their count, mean, smallest and largest in milliseconds, and says
whether the times are those the extension reports or those of the host's own clock.

### Audio

A windowed run plays music, background sounds, music effects and sound effects on the default
output device. The format of a file is recognised from its content: Ogg Vorbis, WAV (PCM) and
MP3 are decoded, and a Standard MIDI file is played with a SoundFont. Without `--soundfont`
the first of `/usr/share/soundfonts/default.sf2`, `/usr/share/soundfonts/FluidR3_GM.sf2` and
`/usr/share/sounds/sf2/FluidR3_GM.sf2` that exists is used (the package `fluid-soundfont-gm`
installs one); it is loaded when the first MIDI file is played.

Nothing about audio stops a game. A file that cannot be decoded, MIDI music without a
SoundFont and an output device that cannot be opened are each reported once on the error
stream, and the game runs on without that sound. A headless run plays nothing and opens no
audio device. Volume is a linear factor for now, not the original's curve, and the loop
markers of MIDI files are not followed: a track loops from its beginning.

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

### Seed, preload scripts and input recordings

Games call `rand`, so two runs only draw the same frames when they start from the same seed.
`--seed` sets it; a headless run without the option uses seed 0, which is why its frames are
reproducible.

A preload script is a patch for a game that is not to be edited: it runs after the `RPG`
module is defined and before the game's first script, so it can define constants and methods
the game then meets. An error in one ends the run like a script error of the game, with the
file's name and line in the report, and a file that does not exist ends it before any script.

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --device RADV \
    --preload fixes/fonts.rb --preload fixes/skip_intro.rb
```

`--record` writes what the game was given as input, frame by frame, and `--replay` gives it to
the game again, in a window or headless:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --device RADV \
    --record session.txt
VK_LOADER_DRIVERS_SELECT='*lvp*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> \
    --headless --replay session.txt --frames 600 --dump 599=target/frame.png
```

A recording is text: the line `vkxp-input 2 seed=<n>`, then one line per run of frames with
the same input, `<count>x` followed by the `+`-joined button names, if any, by `keys=` and the
hexadecimal Windows virtual-key codes of the held keys and mouse buttons, if any, and by
`pointer=<x>,<y>` while the pointer is over the picture. Recordings of version 1, with the
buttons only, still replay. The replay
uses the recorded seed unless `--seed` is given. After the last recorded frame the keyboard
applies again in a window, and no button is held in a headless run. A file that is not a
recording ends the program with an error that names it, before the first frame. While input
is recorded or replayed the game is given each frame's buttons together with that frame's
tick, so a key pressed and released between two frames is not seen. Only input and the seed
are recorded: a reset with F12 is not, and a game that reads the clock can still diverge.

## Comparing with the original runtime

`cargo xtask oracle` runs one game in the original runtime and in vkxp under the same
conditions and reports how chosen frames differ. The original runtime, the run-time package
and the game are yours and stay where they are: the command takes their locations, works on
copies of the game and writes only below its output directory.

```sh
cargo xtask oracle <game directory> --frames 10,20,200 \
    --dll <RGSS DLL> --rtp Standard=<RTP directory> --input "60:RIGHT, 100:"
```

| Option | Meaning |
|---|---|
| `--frames N[,N...]` | The frames to capture, numbered from 0 as `vkxp --dump` numbers them. Required. |
| `--dll FILE` | The RGSS DLL that the game's `Game.ini` names with `Library=`. Required unless `--vkxp-only` is given. |
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`. Repeatable. |
| `--input SCHEDULE\|@FILE` | The buttons held from given frames on, in the syntax described under *Running the demo*, or a file that holds such a schedule. |
| `--builtin-scripts FILE` | The built-in scripts file for vkxp, instead of its default location. |
| `--out DIR` | Where everything is written. Defaults to `target/oracle`. |
| `--vkxp-only` | Run only vkxp. Needs neither Wine nor Xvfb nor the DLL. |
| `--locale LOCALE` | `LC_ALL` of the original, which decides the code page its file names are read in: `zh_CN.UTF-8`, the default, for a Chinese game, `ja_JP.UTF-8` for a Japanese one. |
| `--timeout SECONDS` | How long the original may take to reach one frame. Defaults to 120. |

Each side runs a copy of the game, `<out>/original/game` and `<out>/vkxp/game`, with a probe
script added in front of the scripts of its archive. The probe seeds the random numbers with
a fixed value, answers `Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and
`Input.dir8` from the schedule instead of the keyboard, and counts frames as the host of vkxp
does: one for every `Graphics.update` and `d` for `Graphics.transition(d)`. The game directory
given is never written to, and the output directory may not be inside it.

A game that ships its data in `Game.rgssad` (RGSSAD version 1) is compared as it is: the
probe is added to the script archive inside the encrypted archive of each copy, every other
entry is written back unchanged, and no loose script archive is added, because the original
reads its scripts only from the archive when there is one. To make such a game from an
unpacked one, `cargo xtask pack-rgssad <game directory>` writes every file below the
directory's `Data` and `Graphics` folders into `Game.rgssad`, under its path with
backslashes, and then removes the two folders. It changes the directory it is given, so give
it a copy; it refuses a directory that already has an archive or has no `Game.ini`.

The original side needs `wine`, `wineboot`, `wineserver` and `Xvfb` on `PATH` and a game that
has its `Game.exe`. The command starts a virtual display of its own, makes a Wine prefix at
`<out>/prefix` on the first run (which takes a while), registers the run-time packages and
replacements for common game fonts in it, and runs `Game.exe` with software rendering only.
At each frame asked for the probe holds the game until the command has read the picture of
the game window from the display; after the last one the game ends. A frame that falls
inside a transition is captured when the transition has ended, because the original cannot
be held in the middle of one. What Wine and the game print is in `<out>/original/wine.log`,
which is where to look when the command reports that a frame was not reached. vkxp runs
headless on the CPU device unless the Vulkan loader is already restricted to other drivers.

Text can only be compared when both runtimes draw it with the same font file. `--font FILE`,
which may be given several times, names such a file, a TrueType or OpenType font or a
collection: the files are copied to `<out>/fonts`, which vkxp searches before the fonts of the
system and sizes by their own line boxes (`--font-own-metrics`), as the original does, and into the font directory of the prefix, where they are registered, and the common
game font names (`SimHei`, `黑体`, `SimSun`, `MS PGothic` and the like) are replaced in the
original by the family name read from the first file. A file that cannot be read or is not a
font ends the run before either runtime starts. Without `--font` the original draws those
names with whatever `Noto Sans CJK` the machine has and vkxp with its own choice, and the
command prints a line saying that text is not comparable.

The command prints one line per frame and writes:

| File | Contents |
|---|---|
| `<out>/frames/<n>-original.png` | frame `n` of the original |
| `<out>/frames/<n>-vkxp.png` | frame `n` of vkxp |
| `<out>/frames/<n>-diff.png` | the absolute difference of red, green and blue, multiplied by 32 |
| `<out>/summary.json` | for every frame the three file names, `differing_pixels` and `largest_difference` (of any colour channel) |

It exits with status 0 when both runs completed, whatever the differences are; a missing
prerequisite or a run that fails is an error that names what was missing or what happened.
With `--vkxp-only` there is nothing to compare, and the summary lists the pictures of vkxp.
The pictures show the game's and the run-time package's art, so they stay out of the
repository, like everything else below `target/`.

The tests of the harness run the vkxp side on games they generate. The one test of the
original side is ignored by default, because it needs Wine, Xvfb and files that are not in the
repository; it captures frames 10 and 20 of a real game below `target/oracle-test`. Its three
variables hold absolute paths:

```sh
VKXP_ORACLE_DLL=<RGSS DLL> VKXP_ORACLE_RTP=<RTP directory> VKXP_ORACLE_GAME=<game directory> \
    cargo test -p xtask -- --ignored the_original_is_captured
```

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
