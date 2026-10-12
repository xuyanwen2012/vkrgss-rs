# rgss-runtime Specification

## Purpose
Defines how the guest runs a game: preparing the interpreter, loading and evaluating the game's
scripts, reporting errors, resetting, ending, and the global functions and policies that
scripts meet outside the RGSS classes.

## Requirements

### Requirement: Interpreter preparation
Before any game script runs, `$KCODE` SHALL be `"UTF8"`, the load path SHALL be empty, `$DEBUG`
and `$BTEST` SHALL be `false` unless the caller asks for debug or battle-test mode, and the
window title SHALL have been sent to the host from the game configuration file.

#### Scenario: Globals
- **WHEN** the first game script evaluates `[$KCODE, $:, $DEBUG, $BTEST]`
- **THEN** the result is `["UTF8", [], false, false]`

#### Scenario: Debug mode
- **WHEN** the caller asks for debug mode
- **THEN** `$DEBUG` is `true` in the first game script

#### Scenario: Title
- **WHEN** a game whose configuration file has the title `Demo` is started
- **THEN** the host receives a title message with `Demo` before the first frame

### Requirement: Built-in scripts
The built-in `RPG` scripts SHALL be evaluated from a file named by the caller before the
game's scripts, so that `RPG::Cache`, `RPG::Sprite`, `RPG::Weather` and the `RPG` data classes
are defined. When that file is missing the run SHALL fail before any game script with an error
that names the file and says how it is produced.

#### Scenario: Data classes are present
- **WHEN** a game script evaluates `defined?(RPG::Cache)` after start-up with a built-in scripts file that defines it
- **THEN** the result is `"constant"`

#### Scenario: File missing
- **WHEN** the caller names a built-in scripts file that does not exist
- **THEN** the run fails with an error naming that file, and no game script has run

### Requirement: Script archive
The script archive named by the game configuration file SHALL be read through the game's files
as a `Marshal` array of entries holding an identifier, a name and zlib-compressed source. Each
entry's source SHALL be evaluated in array order under the file name `Section` followed by the
entry's index as three digits, counted from zero. `$RGSS_SCRIPTS` SHALL hold the entries, each
with its decompressed source appended as a fourth element.

#### Scenario: Order
- **WHEN** an archive has three scripts that each append their name to a global array
- **THEN** after the run the array holds the three names in archive order

#### Scenario: Section names
- **WHEN** the second script evaluates `__FILE__`
- **THEN** the result is `"Section001"`

#### Scenario: Script table
- **WHEN** a script evaluates `$RGSS_SCRIPTS[0][3]`
- **THEN** the result is the source text of the first script

#### Scenario: Missing archive
- **WHEN** the configured archive does not exist
- **THEN** the run fails with an error naming the path, and no script has run

### Requirement: End of the game
When the last script returns, or a script raises `SystemExit`, the guest SHALL stop all audio,
send the host its shutdown message and finish successfully.

#### Scenario: Scripts finish
- **WHEN** the last script of the archive returns
- **THEN** the host receives the shutdown message and the run reports success

#### Scenario: Exit
- **WHEN** a script calls `exit`
- **THEN** no later script runs, the host receives the shutdown message and the run reports success

### Requirement: Uncaught errors
An exception that no script rescues SHALL end the run with a failure. Its report SHALL name the
script by the name stored in the archive and the line, in the form
`Script '<name>' line <n>: <class> occurred.` followed by the exception's message, SHALL be
written to the error stream and SHALL be sent to the host as a modal message before the
shutdown message.

#### Scenario: Runtime error
- **WHEN** line 3 of a script named `Scene_Title` raises `NoMethodError`
- **THEN** the report starts with `Script 'Scene_Title' line 3: NoMethodError occurred.` and the run reports failure

#### Scenario: Syntax error
- **WHEN** a script named `Broken` does not parse
- **THEN** the report names `Broken` and `SyntaxError`, and no later script runs

### Requirement: Reset
When the host asks for a reset, the next `Graphics.update` SHALL raise `Reset`. If no script
rescues it, the guest SHALL stop all audio, dispose every sprite, viewport, window, plane,
tilemap and bitmap, and evaluate the game's scripts again from the first, in the same
interpreter.

#### Scenario: Scripts start over
- **WHEN** the host sends a reset request while the main loop of the last script is running
- **THEN** the first script is evaluated again and the scene holds no element from before the reset

#### Scenario: Reset is an exception class
- **WHEN** a script evaluates `Reset.ancestors.include?(Exception)` and `Reset.ancestors.include?(StandardError)`
- **THEN** the results are `true` and `false`

### Requirement: Data files
`load_data(path)` SHALL read the file through the game's files as data and return the object
`Marshal` loads from it, raising `Errno::ENOENT` with the path as given when it is not found.
`save_data(object, path)` SHALL write the object's `Marshal` dump to that path relative to the
game directory.

#### Scenario: Load
- **WHEN** `load_data("Data/Thing.rxdata")` is evaluated and `data/thing.rxdata` holds the dump of `[1, "two"]`
- **THEN** the result is `[1, "two"]`

#### Scenario: Missing
- **WHEN** `load_data("Data/Nope.rxdata")` is evaluated
- **THEN** `Errno::ENOENT` is raised with the message `No such file or directory - Data/Nope.rxdata`

#### Scenario: Save then load
- **WHEN** `save_data({1 => 2}, "Out.rxdata"); load_data("Out.rxdata")` is evaluated
- **THEN** the result is `{1 => 2}`

### Requirement: Message output
`print` SHALL send the host a modal message with its arguments converted by `to_s` and joined,
and `p` one with each argument's `inspect` on its own line; both SHALL also write the text to
the error stream and SHALL return only after the host has dismissed the message.

#### Scenario: Print
- **WHEN** `print("a", 1)` is evaluated
- **THEN** the host receives a modal message with the text `a1`

#### Scenario: Inspect
- **WHEN** `p "a", nil` is evaluated
- **THEN** the host receives a modal message with the text `"a"` and `nil` on separate lines

### Requirement: Process functions are off by default
Unless the caller enables them, `system`, backtick commands, `exec`, `fork`, `syscall` and
`IO.popen` SHALL start nothing, SHALL return `nil` and SHALL write one line naming the call to
the error stream.

#### Scenario: System
- **WHEN** `system("touch should_not_exist")` is evaluated with the default settings
- **THEN** the result is `nil` and no file is created

#### Scenario: Enabled by the caller
- **WHEN** the caller enables process functions and `system("true")` is evaluated
- **THEN** the result is `true`

### Requirement: Loading code
`require` and `load` SHALL load only files with the extension `.rb` that lie inside the game
directory, and SHALL raise `LoadError` for anything else, including compiled extensions.

#### Scenario: Ruby file in the game directory
- **WHEN** the game directory holds `Extra.rb` defining a constant and `require "Extra"` is evaluated
- **THEN** the constant is defined

#### Scenario: Outside the game directory
- **WHEN** `require "/etc/passwd"` or `load "../other.rb"` is evaluated
- **THEN** a `LoadError` is raised

### Requirement: Win32API placeholder
A class `Win32API` SHALL exist whose constructor accepts a library name, a function name, an
argument specification and a return specification, and whose `call` and `Call` return 0. The
first call of each library and function pair SHALL write one line naming it to the error
stream.

#### Scenario: Unknown function
- **WHEN** `Win32API.new("user32", "GetAsyncKeyState", "i", "i").call(13)` is evaluated twice
- **THEN** both calls return 0 and exactly one line naming `user32` and `GetAsyncKeyState` is written

### Requirement: Random seed
When the caller gives a seed, the random number generator SHALL be seeded with it before the
built-in scripts run and again on every reset, so that two runs with the same seed and input
produce the same sequence of `rand` results.

#### Scenario: Same seed
- **WHEN** a game that writes ten values of `rand(1000)` to a file is run twice with seed 7
- **THEN** both files are equal

#### Scenario: Different seeds
- **WHEN** it is run with seed 7 and with seed 8
- **THEN** the files differ

### Requirement: Preload scripts
The files the caller names as preload scripts SHALL be evaluated in the given order after the
built-in scripts and before the game's first script, and again after a reset. An error in a
preload script SHALL end the run like an uncaught script error, with the file's name in the
report. A file that does not exist SHALL fail the run before any script.

#### Scenario: Patch before the game
- **WHEN** a preload script defines a constant and the game's first script reads it
- **THEN** the game sees the constant

#### Scenario: Error in a preload script
- **WHEN** a preload script raises on its line 2
- **THEN** the report names that file and line 2, and no game script runs

### Requirement: Emulated Win32API functions
`Win32API` objects for the functions of the emulation table SHALL behave as the Windows
function does for the uses listed here, whatever the letter case of the library name and with
or without its `.dll` suffix, and SHALL write results into string arguments in place.

#### Scenario: Reading the game configuration file
- **WHEN** `GetPrivateProfileStringA` is called with section `Game`, key `Title`, a default, a buffer of 256 bytes and the file name `.\Game.ini`
- **THEN** the buffer starts with the title in the file's encoding followed by a zero byte, and the return value is the title's length in bytes

#### Scenario: Missing key
- **WHEN** the key does not exist
- **THEN** the buffer holds the default and its length is returned

#### Scenario: Integer value
- **WHEN** `GetPrivateProfileIntA` reads a key whose value is `42`
- **THEN** it returns 42, and the given default for a missing key

#### Scenario: Writing
- **WHEN** `WritePrivateProfileStringA` sets a key in a file inside the game directory and the key is read back
- **THEN** the new value is returned

#### Scenario: Text conversion
- **WHEN** `MultiByteToWideChar` with code page 65001 converts the UTF-8 bytes of `测试` and `WideCharToMultiByte` with code page 936 converts the result
- **THEN** the final bytes are the GBK encoding of `测试`, and each call with a zero-size output returns the size needed

#### Scenario: Screen size and ticks
- **WHEN** `GetSystemMetrics` is called with 0 and with 1, and `GetTickCount` twice with frames in between
- **THEN** the metrics are 640 and 480 or larger, and the second tick count is not smaller than the first

#### Scenario: Window handle
- **WHEN** `FindWindowA` is called with the class `RGSS Player` and `GetClientRect` with the returned handle and a 16-byte buffer
- **THEN** the handle is non-zero and the buffer holds the rectangle 0, 0, 640, 480 as four 32-bit integers

### Requirement: Files reached through Win32API stay in the game directory
Profile functions SHALL read and write only files inside the game directory; a path outside
it SHALL be treated as a missing file for reading and SHALL fail for writing.

#### Scenario: Outside path
- **WHEN** `WritePrivateProfileStringA` names `C:\Windows\win.ini` or `../other.ini`
- **THEN** nothing is written and the return value is 0

### Requirement: Message panel
While `print`, `p` or the report of an uncaught error waits for its dismissal, the guest SHALL
keep sending frames that show the message in a panel drawn above everything else, with the
scene underneath unchanged, and SHALL remove the panel when the dismissal arrives. Text SHALL
be wrapped to the panel's width, and lines beyond the panel's height SHALL be cut with an
ellipsis. When the caller makes `p` log-only, `p` SHALL show no panel and wait for nothing.

#### Scenario: Panel is visible
- **WHEN** a script that shows a red sprite calls `print("hello")` against a host that dismisses after three frames
- **THEN** those frames differ from the frame before the call in the panel's area, are unchanged outside it, and the frame after the dismissal equals the frame before the call

#### Scenario: Error report
- **WHEN** a script raises an uncaught error
- **THEN** frames with a panel are sent before the shutdown message

#### Scenario: Log-only p
- **WHEN** `p 1` is evaluated with `p` made log-only
- **THEN** the text is on the error stream, no modal message is sent and no frame is sent by the call

### Requirement: Win32API name fallback and file functions
A `Win32API` function name that is not in the emulation table SHALL be looked up again with
`A` appended before it is treated as unknown. `CopyFileA` SHALL copy a file when source and
destination are inside the game directory and SHALL return 0 without copying otherwise or when
its third argument is non-zero and the destination exists. `SetPriorityClass` SHALL return 1
and do nothing, and `GetProcessTimes` SHALL fill its four 8-byte buffers with zeros and
return 1.

#### Scenario: Plain name
- **WHEN** `Win32API.new("kernel32", "GetPrivateProfileString", "pppplp", "l")` reads the title from `.\Game.ini`
- **THEN** it behaves as `GetPrivateProfileStringA` and nothing is reported as not emulated

#### Scenario: Copy inside the game directory
- **WHEN** `CopyFileA` is called with `Save\a.rxdata`, `Save\b.rxdata` and 0, and the source exists
- **THEN** the destination has the same bytes and the return value is non-zero

#### Scenario: Copy outside
- **WHEN** `CopyFileA` names a destination outside the game directory
- **THEN** nothing is written and the return value is 0

### Requirement: Keyboard and mouse through Win32API
`GetAsyncKeyState` and `GetKeyState` SHALL return a value with bit 15 set while the given
virtual key is held in the snapshot latched by the last `Input.update`, and
`GetAsyncKeyState` SHALL also set bit 0 when the key was not held in the snapshot before it.
`GetKeyboardState` SHALL fill a 256-byte buffer with 0x80 for each held key. `GetKeyState`
and `GetKeyboardState` SHALL also report in bit 0 a toggle of the key, 0 at the start, that
flips each time a latched snapshot has the key held and the one before had not.

#### Scenario: Held key
- **WHEN** the latched snapshot has virtual key 0x0D held and `GetAsyncKeyState.call(0x0D) & 0x8000` is evaluated
- **THEN** the result is `0x8000`, and 0 for a key that is not held

#### Scenario: Keyboard state
- **WHEN** `GetKeyboardState` is called with a 256-byte buffer while 0x41 is held for the first time
- **THEN** byte 0x41 of the buffer is 0x81 and the others are 0

#### Scenario: Toggle
- **WHEN** key 0x14 is held in one latched snapshot, released in the next and held again in a third
- **THEN** `GetKeyState.call(0x14) & 1` is 1 after the first, 1 after the second and 0 after the third

### Requirement: Pointer through Win32API
`GetCursorPos` SHALL write the pointer position as two 32-bit integers, and `ScreenToClient`
SHALL leave a point unchanged, so that the two together give frame coordinates; without a
pointer the position SHALL be (-1, -1).

#### Scenario: Pointer
- **WHEN** the snapshot's pointer is (100, 50) and `GetCursorPos` is called with an 8-byte buffer
- **THEN** the buffer unpacks with `"ll"` to `[100, 50]`

### Requirement: Key injection functions are accepted
`keybd_event` SHALL send the host a press of the given virtual key, or a release when bit 1
of its flags is set. `mouse_event` SHALL send a press or release of key 0x01, 0x02 or 0x04
for each button flag it is given. Both SHALL return 0. `SetKeyboardState` with a 256-byte
buffer SHALL replace what `GetKeyState` and `GetKeyboardState` report, the held bits until
the next `Input.update`, and SHALL return 1. None SHALL be reported as not emulated.

#### Scenario: keybd_event
- **WHEN** `keybd_event` is called with `(0x0D, 0, 0, 0)` and then with `(0x0D, 0, 2, 0)`
- **THEN** the guest has sent a press and then a release of key 0x0D, and nothing is written to the error stream

#### Scenario: mouse_event
- **WHEN** `mouse_event` is called with the flags `0x02 | 0x10`
- **THEN** the guest has sent a press of key 0x01 and a release of key 0x02

#### Scenario: SetKeyboardState
- **WHEN** `SetKeyboardState` is called with a buffer whose byte 0x41 is 0x80 and the others 0, while key 0x10 is held
- **THEN** `GetKeyState.call(0x41)` has bit 15 set and `GetKeyState.call(0x10)` is 0, until `Input.update` is called, after which it is the other way round

### Requirement: Saving a state
A guest that is asked to save a state SHALL have itself copied at a wait for a tick whose
frame was sent while the display was not frozen and no modal message was shown, and SHALL go
on unchanged. A guest that cannot be copied SHALL answer that the request failed.

#### Scenario: Unchanged run
- **WHEN** a game is run twice with the same seed and input, once with a save request at frame 50
- **THEN** the frames the guest sends are the same in both runs

#### Scenario: Frozen display
- **WHEN** the request arrives between `Graphics.freeze` and the end of the transition that follows
- **THEN** the copy is made at the first frame after the transition

#### Scenario: No copier
- **WHEN** the guest runs without a way to copy itself, as a thread does
- **THEN** it answers the request with the failure message and goes on

### Requirement: Resuming a state
A resumed copy SHALL send its title, the play commands of the music and background sound it
remembers and the frame it was waiting on, with every live bitmap and table in full, and SHALL
then continue from that wait.

#### Scenario: Complete frame
- **WHEN** a copy made while three bitmaps and a tilemap's tables were live is resumed
- **THEN** the first frame it sends creates and fills the three bitmaps and sets the tables, and a compositor that held nothing renders it like the frame of the original wait

#### Scenario: Music
- **WHEN** the game was playing a music file when the copy was made
- **THEN** the resumed copy sends a play command for that file before its frame

### Requirement: The reset stays the current error
While the scripts are evaluated again after a reset, `$!` SHALL be the `Reset` exception
that ended the previous run and `$@` its backtrace, until a script replaces them. Before the
first run both SHALL be nil.

#### Scenario: First run
- **WHEN** the first script records `$!` and `$@` at its start
- **THEN** both are nil

#### Scenario: After a reset
- **WHEN** the host sends a reset request during `Graphics.update` and the first script records `$!.class` and `$@` when it is evaluated again
- **THEN** the class is `Reset` and `$@` is an array whose first entry names `update`

#### Scenario: Guarded alias
- **WHEN** a script wraps `Sprite#dispose` with `alias old_dispose dispose unless $@`, the game is reset twice and a sprite is then disposed
- **THEN** the sprite is disposed without an error

#### Scenario: Rescue inside the second run
- **WHEN** a script of the run after a reset raises and rescues an error of its own and a later script records `$!.class`
- **THEN** the class is `Reset`

### Requirement: Pointer visibility through Win32API
`ShowCursor` SHALL keep its display count and return it, and SHALL tell the host that the
pointer is to be hidden when the count falls below 0 and that it is to be seen when it
comes back to 0.

#### Scenario: Hide and show
- **WHEN** `ShowCursor.call(0)` is called twice and then `ShowCursor.call(1)` twice
- **THEN** the returns are -1, -2, -1 and 0, and the guest has sent exactly two messages: hidden after the first call, to be seen after the last
