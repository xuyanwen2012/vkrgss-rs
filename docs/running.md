# Running vkxp

How to run a game, every option and configuration key, and the demo that needs no game.
Building is described in [building.md](building.md).

## Running a game

The commands below use `cargo run`, which builds the development profile.
`cargo build --release -p vkxp` produces the optimised executable, `target/release/vkxp`, which
takes the same arguments.

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
| `--present-stats` | Write the intervals between the frames shown to the error stream. See *Window*. |
| `--frame-stats` | Write where the frames spend their time to the error stream. See *Frame times*. |
| `--verbose` | Also write informational and debug messages to the error stream. See *Diagnostics*. |
| `--no-audio` | Run without sound. No audio device is opened. |
| `--soundfont FILE` | The SoundFont that MIDI music is played with. A file that does not exist ends the program with an error before any window opens. |
| `--seed N` | The seed of the random number generator, set before any script runs and again on every reset. Without it a headless run uses 0, a replay the seed of its recording, and a windowed run one taken from the clock. |
| `--preload FILE` | A Ruby file evaluated after the built-in scripts and before the game's first script, and again after a reset. Repeatable, in the order given. |
| `--record FILE` | Write the input of every frame and the seed to `FILE`. |
| `--replay FILE` | Play the input of a recording back instead of the keyboard. Not together with `--record` or `--input`. |
| `--guest thread\|process` | Where the game's scripts run: on a thread of the program or in a process of their own, which the save states need. Without a choice it is a process in a window and a thread in a headless run. See *Guest process* and *Save states*. |
| `--headless` | Run without a window, on a virtual clock that never waits. |
| `--frames N` | Headless: stop after N frames if the game has not ended. |
| `--input SCRIPT\|@FILE` | Headless: the buttons held from given frames on, in the syntax described under *Running the demo*, or a file that holds such a script. |
| `--dump FRAME=FILE` | Headless: write frame `FRAME`, counted from 0, as a PNG file. Repeatable. |
| `--states SCHEDULE` | Headless, with `--guest process`: save and load states at given frames, for example `"120:save 0, 200:load 0"`. See *Save states*. |
| `--version`, `-V` | Print the name and the version to the standard output and exit. |
| `--help`, `-h` | Print the usage text to the standard output and exit. |

An unknown option or a malformed value, a malformed input script file included, ends the
program with the usage message and status 2.
The exit status is 0 when the game ends normally: its scripts return or call `exit`, the
window is closed, or the frame limit of a headless run is reached. It is non-zero when a
script error ends the game (its report is on the error stream), when the host fails, when a
frame is rejected in a headless run, or when a frame that was to be dumped was never reached.

### Diagnostics

What the running program has to say about itself goes to the error stream as lines that start
with `vkxp: `. By default these are warnings and errors: a sound that cannot be decoded, MIDI
without a SoundFont, no gamepad support, a frame that cannot be drawn. `--verbose` adds
informational and debug messages, which name the choices the program made: the GPU and how
frames are paced, the directories of the run-time packages, the built-in scripts file, the
encrypted archive and the configuration files that were read. Each line then also names its
level and the module it comes from:

```text
vkxp: INFO vkxp: built-in scripts file: /home/me/.local/share/vkxp/rpg_builtin.rb
```

The environment variable `VKXP_LOG` sets the level instead and comes before `--verbose`. It
holds a level (`off`, `error`, `warn`, `info`, `debug` or `trace`) or a filter by module,
such as `VKXP_LOG=warn,vkxp_host=trace`. A value that cannot be parsed ends the program with
the usage message and status 2. There is no key for the level in a configuration file.

What the game itself writes (`print`, `p`, the report of a script error), the usage message
and the message of an error that ends the program are not diagnostics and are shown whatever
the level. The intervals that `--present-stats` asks for and the frame times that
`--frame-stats` asks for are warnings, so they need no `--verbose`.

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
guest = "thread"                             # --guest: "thread" or "process"
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
(F12, F2, F3, F4, F11, Tab, Alt+Enter) are not part of the map.

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
loads the driver of every GPU in the machine unless it is restricted. The command above
restricts it to RADV, the driver of AMD GPUs; the same on the CPU, with lavapipe, is
`VK_LOADER_DRIVERS_SELECT='*lvp*'` and `--device llvmpipe`.

The host keys are those of the demo: F12 (reset), Alt+Enter or F11 (borderless fullscreen),
F2 (frame rate in the title) and Tab held (fast-forward), and the keys of the save states: F3
saves, F4 loads, and with Shift they choose the slot (see *Save states*). Closing the window
asks the game to quit.

After a reset the scripts are evaluated again with the `Reset` exception as `$!` and its
backtrace as `$@`, as in the original runtime, so that the common guard
`alias old_name name unless $@` makes its alias only once.

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
Pacing with `VK_EXT_present_timing` is used whenever the device and surface support it; the
environment variable `VKXP_PRESENT_TIMING=0` switches it off, and then frames are paced by the
host's timer alone. `--present-stats` writes the intervals between the frames shown to the error stream, every
600 frames and at exit, as their count, mean, smallest and largest in milliseconds, and says
whether the times are those the extension reports or those of the host's own clock.

A window that is hidden, or a locked session, may stop handing out images to draw into. The
game then goes on without showing its frames: a present waits at most a tenth of a second for
an image, and the ones after it do not wait at all until the window gives images again. Where
the device offers `VK_EXT_swapchain_maintenance1`, it is used to replace the swapchain, as
after a resize, without idling the whole device.

### Frame times

`--frame-stats` measures where each frame's time goes and writes a summary to the error
stream: every 600 frames and at exit in a window, once at the end of a headless run. Nothing
is measured without the option, and measuring changes neither a frame nor when it is shown.

```text
vkxp: frame times over 600 frame(s):
vkxp:   guest: 600 frame(s), mean 3.412 ms, smallest 2.870 ms, largest 9.105 ms
vkxp:   encode: 600 frame(s), mean 0.031 ms, smallest 0.024 ms, largest 0.112 ms
vkxp:   bin: 600 frame(s), mean 0.018 ms, smallest 0.012 ms, largest 0.094 ms
vkxp:   upload: 600 frame(s), mean 0.142 ms, smallest 0.021 ms, largest 4.730 ms
vkxp:   submit: 600 frame(s), mean 0.655 ms, smallest 0.580 ms, largest 1.904 ms
vkxp:   present: 600 frame(s), mean 0.410 ms, smallest 0.352 ms, largest 1.218 ms
vkxp:   compose (device): 600 frame(s), mean 0.207 ms, smallest 0.180 ms, largest 0.655 ms
vkxp:   present (device): 600 frame(s), mean 0.058 ms, smallest 0.049 ms, largest 0.201 ms
```

The numbers above only show the layout. Each row gives the number of frames the stage was
measured for and its mean, smallest and largest duration in milliseconds:

| Row | What it measures |
|---|---|
| `guest` | The host's wait from answering a frame to receiving the next one: the game's scripts, its bitmap operations and the extraction of the frame. The host holds back its answer until the frame's period is over, so the game's side never sleeps and this is its work alone, in a window as in a headless run. The first frame follows no answer and is not counted. The guest's own rows, below, split this work. |
| `encode` | Checking the frame's ops and encoding them for the shader. |
| `bin` | Assigning the ops to the 16x16 tiles they touch. |
| `upload` | Copying the changed bitmap pixels and the frame's data into buffers and recording the commands. |
| `submit` | Submitting the commands and waiting until the device has finished them: the bitmap uploads and the compose dispatch. |
| `present` | Recording the pass that scales the frame into the window, submitting it and waiting for it. It does not include the wait for a swapchain image or for the frame's turn on the display. |
| `compose (device)` | Time on the device from the start to the end of the compose dispatch, from timestamps the device writes. |
| `present (device)` | The same for the pass that scales the frame into the window. |

The first five stages after `guest` are processor time on the host's thread. The two device
rows count no frames on a device whose queue has no timestamps, and a headless run presents
nothing, so its `present` rows count none either. On llvmpipe the device is the processor
and its timestamps say little.

The guest measures its own frames as well and writes a second block at the same moments,
every 600 frames and when the game's side ends, a little before or after the host's:

```text
vkxp: guest frame times over 600 frame(s):
vkxp:   guest scripts: 600 frame(s), mean 2.104 ms, smallest 1.730 ms, largest 6.412 ms
vkxp:   guest bitmap operations: 600 frame(s), mean 0.611 ms, smallest 0.000 ms, largest 2.250 ms
vkxp:   guest text: 600 frame(s), mean 0.402 ms, smallest 0.000 ms, largest 1.981 ms
vkxp:   guest extraction: 600 frame(s), mean 0.118 ms, smallest 0.094 ms, largest 0.530 ms
```

A frame of the guest lasts from the moment the wait for one frame's tick returns to the start
of the wait for the next, so the wait itself is in no row, and what the game does before its
first frame is not counted. Every row counts every such frame, one in which the part took no
time too.

| Row | What it measures |
|---|---|
| `guest scripts` | The frame without the three parts below: the game's scripts in the interpreter, and everything else the guest does for them, such as loading and decoding a picture. |
| `guest bitmap operations` | The time inside the `Bitmap` methods `blt`, `stretch_blt`, `fill_rect`, `clear`, `hue_change`, `get_pixel` and `set_pixel`. |
| `guest text` | The time inside `Bitmap#draw_text` and `Bitmap#text_size`. |
| `guest extraction` | Extracting the frame from the scene, with the changed bitmap pixels, and handing it to the host. |

A call inside another measured call belongs to the outer one, so no time is counted twice.
A measured call that runs across a frame update, which the conversion of an argument can
cause, gives each frame the part that lies in it, without the wait for the tick.
For what the game's side costs, read these four rows: the host's `guest` row is a wait seen
from the other side, and in a headless run the guest is often ahead of the host, so that row
is smaller than their sum. The clock is read for this report only; no script can see it.

The four rows are not split by method. To see more of where the game's side spends its time,
build with the `profiling` profile, which is the release build with full debug information, and
use a profiler. With `perf`, on a headless run, which does not wait between frames:

```sh
cargo build -p vkxp --profile profiling
VK_LOADER_DRIVERS_SELECT='*lvp*' perf record --call-graph dwarf -o target/perf.data \
    target/profiling/vkxp <game directory> --rtp Standard=<RTP directory> \
    --headless --frames 600
perf report -i target/perf.data --sort dso,symbol
```

The interpreter's thread, `ruby`, carries the scripts, the bitmap operations and the frame
extraction; with llvmpipe the compositing shows up as well, on its own worker threads, and is
no measure of a GPU.

When `VK_EXT_debug_utils` is installed, the backend names its Vulkan objects (the frame
target, the held frame, the frame's buffers, the pipelines, the swapchain images and each
bitmap image by its slot) and labels the upload, compose and present parts of its command
buffers, so a capture in RenderDoc and the messages of the validation layer show names
instead of handles.

### Audio

A windowed run plays music, background sounds, music effects and sound effects on the default
output device. The format of a file is recognised from its content: Ogg Vorbis, WAV (PCM) and
MP3 are decoded, a Standard MIDI file is played with a SoundFont, and WMA is decoded by the
`ffmpeg` program, which is looked up on the search path each time such a file is played.
Without `--soundfont` the first of `/usr/share/soundfonts/default.sf2`,
`/usr/share/soundfonts/FluidR3_GM.sf2` and `/usr/share/sounds/sf2/FluidR3_GM.sf2` that exists
is used (the package `fluid-soundfont-gm` installs one); it is loaded when the first MIDI
file is played.

Nothing about audio stops a game. A file that cannot be decoded, a WMA file without
`ffmpeg`, MIDI music without a SoundFont and an output device that cannot be opened are each
reported once on the error stream, and the game runs on without that sound. A headless run
plays nothing and opens no audio device. Volume follows the original's curve, 0.35 dB below
full for every unit under 100 and silence at 0, and a fade lowers it evenly. Music and
background sound loop; a MIDI track that marks the start of its loop with controller 111, as
music written for RPG Maker does, continues there, and one without the marker at its
beginning.

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

### Guest process

The game's scripts, with the interpreter, the bitmaps and the game's files, are the guest;
the window, the GPU, the audio device and the input are the host. The guest is a thread of
the program or a child process. Without a choice it is a process in a window, where the keys
of the save states need it, and a thread in a headless run. `--guest thread` and
`--guest process` choose:

```sh
VK_LOADER_DRIVERS_SELECT='*radeon*' \
    cargo run -p vkxp -- <game directory> --rtp Standard=<RTP directory> --guest process
```

The environment variable `VKXP_GUEST` and the key `guest` of a configuration file name the
form too; the command line comes before the environment and the environment before the
files. Any value but `thread` and `process` is an error.

The program starts itself again for the guest, with its own arguments, and the two exchange
the messages of the protocol over a Unix stream socket in a directory only the user can
enter, below `$XDG_RUNTIME_DIR` or, without that variable, below the temporary directory. The
directory is removed when the run ends; one that a run ended by a signal left behind is
removed the next time a guest process starts. The guest process opens no window, no Vulkan device
and no audio device.

A run does and shows the same in both forms: the same frames, the same reports on the error
stream, the same exit status. What differs:

- A crash of the interpreter ends the guest process and not the window with it; the program
  reports the signal that ended the guest and exits with a non-zero status.
- The guest process is ended by the kernel should the program itself be killed.
- With `--frame-stats` the guest's own rows are written by the guest process.
- A bitmap that changes is copied through the socket instead of handed over.

The save states, below, build on this form. A sandbox for the guest process would too and
does not exist yet.

### Save states

A state is the game as it is at one frame: everything the scripts hold, whatever the game's
own save system would keep or not. It works in any game, because a state is a copy of the
guest process, made with `fork` at a frame boundary and kept in memory until it is loaded. So states need the guest in a process, which is what a window has
unless `--guest thread` says otherwise.

| Key | |
|---|---|
| F3 | Save a state into the chosen slot. |
| F4 | Load the state of the chosen slot. |
| Shift+F3 | Choose the next slot. |
| Shift+F4 | Choose the slot before. |

There are ten slots, 0 to 9, and slot 0 is chosen at the start. What became of a request, and
the slot that was chosen, is shown in the window title for two seconds and logged at the level
`info`. A state is saved at the next frame at which it can be: not between `Graphics.freeze`
and the end of the transition that follows, and not while a message panel is shown. A slot
can be loaded any number of times. Loading starts the display and the pacing afresh for the
game that goes on from the state, and the music and the background sound go on where they
were when the state was saved; a fade that was running then is not brought back.

The keys only show a notice when the guest is a thread, and while input is recorded or
replayed, since a load would make the recording describe another run. With
`--allow-process-functions` no state is saved: a game that starts processes has to wait for
them, and a guest that keeps copies of itself cannot.

A headless run takes its requests from `--states`, for the tests: entries `frame:save N` and
`frame:load N` separated by commas, made when that frame of the run arrives. The frames of a
run keep counting through a load, so after `--states "60:save 0, 100:load 0"` frame 101 is
the frame that followed frame 60. Without `--guest process` the option is an error.

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo run -p vkxp -- <game directory> \
    --rtp Standard=<RTP directory> --headless --guest process --frames 140 \
    --states "60:save 0, 100:load 0" --dump 101=target/frame.png
```

Limits:

- States live in memory and end with the run. Nothing is written to disk.
- Files the game wrote after a state was saved stay as they are, its own saves included; a
  file a script holds open across frames shares its position with the copies.
- Each state costs the memory pages the game has changed since it was saved.
- States need `fork`, which the one platform of vkxp, Linux, has.

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

## Running the demo

The host can also be driven without Ruby, by a guest written in Rust, `vkxp_host::demo`. It
moves a sprite with the arrow keys (faster with Shift), tints the middle of the screen while
Space, Enter or C is held, and swaps the background with a transition on Escape or X.
The module is compiled only with the feature `testing` of `vkxp-host`, which the crate's examples
and tests enable by themselves: the commands below need no flag.

In a window:

```sh
cargo run -p vkxp-host --example demo -- <device>
```

`<device>` is the index of a Vulkan device or a part of its name, as `vulkaninfo --summary`
prints them. It has no default, so that the GPU that drives the window is always a choice. The
host keys are F12 (reset), Alt+Enter or F11 (borderless fullscreen), F2 (frame rate in the
title) and Tab held (fast-forward). Closing the window asks the guest to quit.

The Vulkan loader loads every installed driver to enumerate the devices, so on a machine
where a GPU is to be left alone the loader is restricted to the driver of the chosen device
with `VK_LOADER_DRIVERS_SELECT`: RADV for an AMD GPU, for example, or lavapipe for the CPU:

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

For example, frame 12 of this run shows the sprite after it has moved, the screen in the
middle of the transition after B, and the tint of C over the middle of the screen:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' \
    cargo run -p vkxp-host --example demo-headless -- --frames 30 \
    --input "0:RIGHT+UP+SHIFT, 6:B, 7:RIGHT+C" --dump 12=target/demo.png
```
