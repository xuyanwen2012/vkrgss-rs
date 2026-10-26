# rgss-runtime Specification

## Purpose
Defines how the guest runs a game: preparing the interpreter, loading and evaluating the game's
scripts, reporting errors, resetting, ending, and the global functions and policies that
scripts meet outside the RGSS classes.

## Requirements

### Requirement: Interpreter preparation
Before any game script runs, `$KCODE` SHALL be `"UTF8"`, the load path SHALL be empty and the
window title SHALL have been sent to the host from the game configuration file. In RGSS1
`$DEBUG` SHALL be `true` in debug mode and `$BTEST` in battle-test mode. In RGSS2 `$TEST`
SHALL be `true` in debug mode, `$TEST` and `$BTEST` in battle-test mode, and `$DEBUG` SHALL
stay `false`. Otherwise these globals SHALL be `false`.

#### Scenario: Globals
- **WHEN** the first game script evaluates `[$KCODE, $:, $DEBUG, $BTEST]`
- **THEN** the result is `["UTF8", [], false, false]`

#### Scenario: Debug mode
- **WHEN** the caller asks for debug mode
- **THEN** `$DEBUG` is `true` in the first game script

#### Scenario: Title
- **WHEN** a game whose configuration file has the title `Demo` is started
- **THEN** the host receives a title message with `Demo` before the first frame

#### Scenario: Globals of VX, measured on the original
- **WHEN** the first script of a game of RGSS2 evaluates `[$TEST, $BTEST, $DEBUG]` in a plain run, in debug mode and in battle-test mode
- **THEN** the results are `[false, false, false]`, `[true, false, false]` and `[true, true, false]`

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
Unless the caller enables them, `system`, backtick commands, `exec`, `fork`, `syscall`,
`IO.popen` and `Process.kill`, and `Kernel#open`, `IO.read`, `IO.foreach`, `IO.readlines`
and, where it exists, `IO.binread` with a name that starts with `|`, SHALL start and signal
nothing, SHALL return `nil` and SHALL write one line naming the call to the error stream.
These functions with any other name SHALL work as before.

#### Scenario: System
- **WHEN** `system("touch should_not_exist")` is evaluated with the default settings
- **THEN** the result is `nil` and no file is created

#### Scenario: Enabled by the caller
- **WHEN** the caller enables process functions and `system("true")` is evaluated
- **THEN** the result is `true`

#### Scenario: Commands through open and IO
- **WHEN** a game of RGSS1 and a game of RGSS3, each run with `--sandbox off`, evaluate `open("|touch a")`, `IO.read("|touch b")`, `IO.readlines("|touch c")`, `IO.foreach("|touch d") {}` and `File.read("|touch e")`
- **THEN** each result is `nil`, none of the files `a` to `e` exists, and the error stream has one line for each call

#### Scenario: Files through open and IO
- **WHEN** the same games write `note.txt` with `open("note.txt", "w")` and read it back with `IO.read("note.txt")` and `IO.readlines("note.txt")`
- **THEN** the reads return what was written

#### Scenario: Process.kill
- **WHEN** a game evaluates `Process.kill(0, Process.ppid)` with the default settings
- **THEN** the result is `nil` and the error stream names `Process.kill`

### Requirement: Loading code
`require` and `load` SHALL load only files with the extension `.rb` that lie inside the game
directory, and SHALL raise `LoadError` for anything else, including compiled extensions.
`autoload` SHALL load only what `require` may. `require` SHALL record a file once it has
loaded, so that a file that raised can be required again. `load` with `wrap` SHALL evaluate
the file in an anonymous module.

#### Scenario: Ruby file in the game directory
- **WHEN** the game directory holds `Extra.rb` defining a constant and `require "Extra"` is evaluated
- **THEN** the constant is defined

#### Scenario: Outside the game directory
- **WHEN** `require "/etc/passwd"` or `load "../other.rb"` is evaluated
- **THEN** a `LoadError` is raised

#### Scenario: autoload over Ruby 1.8
- **WHEN** a game of RGSS1 evaluates `autoload(:Far, "/etc/passwd")`, and `autoload(:Lazy, "Lazy")` with `Lazy.rb` in the game directory before it uses `Lazy`
- **THEN** the first raises `LoadError` at the call, and `Lazy` is defined at its first use

#### Scenario: A file that raised is required again
- **WHEN** `Flaky.rb` raises the first time it is evaluated and defines a constant the second time, and a script evaluates `(require "Flaky" rescue nil)` and then `require "Flaky"`
- **THEN** the second `require` returns `true` and the constant is defined, over Ruby 1.8.7 and over Ruby 1.9.2

#### Scenario: A file that requires itself
- **WHEN** `Self.rb` counts its evaluations and evaluates `require "Self"`, and a script requires it
- **THEN** it was evaluated once

#### Scenario: Wrapped load
- **WHEN** `Part.rb` defines the constant `PART` and a script evaluates `load("Part.rb", true)`
- **THEN** the result is `true` and `defined?(PART)` at the top level is `nil`

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
and `GetKeyboardState` SHALL also report in bit 0 a toggle, 0 at the start, that flips each
time a latched snapshot newly has the key held.

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

### Requirement: Symbol numbers follow the runtime
The program SHALL start the interpreter with the start-up symbol table of the run's runtime,
so that the scripts of a game see the symbol numbers of the original runtime it was made for.

#### Scenario: First new names of a game of VX, measured on the original
- **WHEN** the first script of a game of RGSS2 evaluates `[:zz_first.to_i, :zz_second.to_i]`
- **THEN** the result is `[16089, 16097]`

#### Scenario: Known names of a game of VX, measured on the original
- **WHEN** a script of a game of RGSS2 evaluates `[:initialize.to_i, :to_s.to_i, :update.to_i]`
- **THEN** the result is `[2961, 3161, 6481]`

#### Scenario: XP is unchanged
- **WHEN** the existing test of the symbol numbers of a game of RGSS1 runs
- **THEN** it passes without a change to its expected numbers

### Requirement: Names interned at first use in RGSS2
In a game of RGSS2 the runtime SHALL intern, at the moments the original runtime does and in
its order, the names `owner` and `observer`, the six names `__hdll__`, `__dll__`,
`__dllname__`, `__proc__`, `__import__`, `__export__` of `Win32API`, and `bmp0` to `bmp8` of a
tilemap's bitmap slots, and SHALL intern no name of its own at run time that takes a number
of the sequence.

#### Scenario: First uses, measured on the original
- **WHEN** a game of RGSS2 notes the number of a new name before and after each of `Color.new(0, 0, 0)`, `Bitmap.new(1, 1)`, `Sprite.new`, and `Win32API.new("kernel32", "GetTickCount", "", "L")`
- **THEN** the number advances by 0, 2, 0 and 6 beyond the name noted

#### Scenario: Tilemap, measured on the original
- **WHEN** a game of RGSS2 notes the number of a new name before and after `Tilemap.new`, then after `t.bitmaps[3]`, and after `t.bitmaps[3] = nil`
- **THEN** the number advances by 0, by 1 and by 0 beyond the name noted

#### Scenario: Every measured step
- **WHEN** the steps of the committed probe of first uses run in a game of RGSS2
- **THEN** the number of a new name after each step equals the number measured on the original, except at the steps the test lists with their reason

#### Scenario: XP
- **WHEN** the existing test of the symbol numbers of a game of RGSS1 runs
- **THEN** it passes without a change to its expected numbers

### Requirement: Preparation of a game of RGSS3
Before the first script of a game of RGSS3 the globals SHALL be those measured on the
original runtime, `RGSSReset` SHALL be a class under `Exception`, `RGSS_VERSION` SHALL be
`"3.0.1"`, and neither `Reset` nor `Hangup` SHALL be defined. This SHALL hold in the place
of the requirement "Interpreter preparation" where the two differ. The binding over Ruby
1.8.7 SHALL refuse to run a game of RGSS3.

#### Scenario: Globals, measured on the original
- **WHEN** the first script of a generated game of RGSS3 evaluates `[$KCODE, $:, $TEST, $BTEST, $DEBUG]` in a plain run and in debug mode
- **THEN** the results are `[nil, ["."], false, false, false]` and `[nil, ["."], true, false, false]`

#### Scenario: The reset class, measured on the original
- **WHEN** it evaluates `[RGSSReset.superclass, RGSSReset.new.message, defined?(Reset), defined?(Hangup)]`
- **THEN** the result is `[Exception, "RGSSReset", nil, nil]`

#### Scenario: Version
- **WHEN** it evaluates `[RGSS_VERSION, RGSS_VERSION.encoding.name]`
- **THEN** the result is `["3.0.1", "ASCII-8BIT"]`

#### Scenario: Title
- **WHEN** a game of RGSS3 whose configuration file has the title `Demo` is started
- **THEN** the host receives a title message with `Demo` before the first frame

#### Scenario: A game of RGSS3 over Ruby 1.8.7
- **WHEN** `run_game` of the binding over Ruby 1.8.7 is given a game of RGSS3
- **THEN** it returns an error that names the runtime and the interpreter, and nothing of the game was evaluated

#### Scenario: Games of RGSS1 and RGSS2 over Ruby 1.9.2 in the tests
- **WHEN** the shared tests of the binding run over Ruby 1.9.2
- **THEN** they prepare, load and report their games of RGSS1 and RGSS2 as before this change, and the allow-list of that package gains no entry

### Requirement: Built-in scripts of RGSS3
In a game of RGSS3 the built-in scripts file SHALL be evaluated whole before the script
archive is read. `rgss_main` and `rgss_stop` SHALL be what that file defines: the binding
SHALL define neither, and the repository SHALL hold no text of them but stand-ins written
for its tests.

#### Scenario: The three blocks
- **WHEN** a game of RGSS3 is started with a built-in scripts file that defines the module `RPG`, `rgss_main` and `rgss_stop`, and its script evaluates `[defined?(RPG), respond_to?(:rgss_main, true), respond_to?(:rgss_stop, true)]`
- **THEN** the result is `["constant", true, true]`

#### Scenario: A file without the functions
- **WHEN** the file defines only the module `RPG` and the script calls `rgss_main { }`
- **THEN** the run ends with a report of a `NoMethodError` that names the script

#### Scenario: The stand-in is not the original text
- **WHEN** the stand-in of the tests is read
- **THEN** it says in a comment that it was written for the tests of vkrgss, and it is the text given in this change's `design.md`

### Requirement: Script archive of RGSS3
In a game of RGSS3 `$RGSS_SCRIPTS` SHALL be what `load_data` returns for the script archive,
each entry with its inflated source inserted at index 3. The entries counted at that moment
SHALL be evaluated in order on the main object, each from `$RGSS_SCRIPTS[i][3]` as it is when
its turn comes, compiled as UTF-8, under the file name `{` + the index as four digits + `}`.
This SHALL hold in the place of the requirement "Script archive".

#### Scenario: Order and names, measured on the original
- **WHEN** an archive has three scripts that each append `__FILE__` to a global array
- **THEN** after the run the array is `["{0000}", "{0001}", "{0002}"]`

#### Scenario: Encodings, measured on the original
- **WHEN** the second script, whose source holds the literal `"測"` in UTF-8, evaluates `[__ENCODING__.name, "測".encoding.name, "測".size, __FILE__.encoding.name, $RGSS_SCRIPTS[1][3].encoding.name]`
- **THEN** the result is `["UTF-8", "UTF-8", 1, "ASCII-8BIT", "ASCII-8BIT"]`

#### Scenario: Script table, measured on the original
- **WHEN** an archive whose second entry is `[7, "Second", <deflated source>, :extra]` is loaded and a script evaluates `$RGSS_SCRIPTS[1]`
- **THEN** it is an array of five whose first two values are `7` and `"Second"`, whose value at index 3 is the source text and whose value at index 4 is `:extra`, and the name and the deflated source are labelled UTF-8

#### Scenario: The source is read when its turn comes, measured on the original
- **WHEN** the first of three scripts assigns `$RGSS_SCRIPTS[1][3] = "$seen = :changed"` and appends a fourth entry whose source sets `$extra`
- **THEN** after the run `$seen` is `:changed` and `$extra` is `nil`

#### Scenario: What passes from script to script, measured on the original
- **WHEN** the first script evaluates `local = 1; @ivar = 2; CONST = 3; def helper; 4; end` and the second `[defined?(local), @ivar, CONST, helper, self.to_s]`
- **THEN** the result is `[nil, 2, 3, 4, "main"]`

#### Scenario: Both forms of the strings
- **WHEN** a game whose archive holds its names and deflated sources with `:E` true, as the editor writes them, and the same game with those strings plain are run
- **THEN** both run their scripts, and in both the name of an entry is labelled UTF-8

#### Scenario: Line ends and empty scripts
- **WHEN** an archive holds an empty script, a script of comments only and a script with CR LF line ends that sets a global on its third line
- **THEN** the run ends with success and the global is set

#### Scenario: Missing archive
- **WHEN** the configured archive does not exist, or holds bytes that are no `Marshal` dump
- **THEN** the run fails with an error naming the path, and no script has run

### Requirement: End of a game of RGSS3
A game of RGSS3 SHALL end with success when its last script returns and when a script leaves
by `exit` with any argument, by `raise SystemExit` or by `abort`; the guest SHALL then stop
all audio and send the shutdown message, and the exit status of the program SHALL be 0, as
measured on the original.

#### Scenario: Exit with a status
- **WHEN** the scripts of four generated games of RGSS3 end with `exit`, `exit(3)`, `exit(false)` and `abort`
- **THEN** each run exits with status 0, and no later script has run

#### Scenario: Exit inside the block of the main function and inside a Fiber
- **WHEN** a script calls `exit` inside `rgss_main { }` and another inside a resumed Fiber
- **THEN** each run exits with status 0

#### Scenario: A loop driven by a Fiber
- **WHEN** the script of a game of RGSS3 moves a sprite one step per resume of a Fiber that yields after each step, with `Graphics.update` between the resumes, for ten frames, and frames 2 and 9 are dumped
- **THEN** the exit status is 0 and the sprite is further along in the second picture than in the first

### Requirement: Uncaught errors of RGSS3
An exception that no script of a game of RGSS3 rescues SHALL end the run with a failure and
a report in the form measured on the original, written to the error stream and sent to the
host as a modal message before the shutdown message. The script SHALL be named by
`$RGSS_SCRIPTS[i][1]` as it is then, and the position SHALL be that of the first backtrace
entry outside `(eval)`. The exit status SHALL be 1.

#### Scenario: Runtime error
- **WHEN** line 3 of the script at index 1, named `Scene_Title`, calls an undefined method on `nil`
- **THEN** the report starts with `Script 'Scene_Title' line 3: NoMethodError occurred.`, an empty line follows and then the message of the exception

#### Scenario: A method of another script
- **WHEN** the last script calls a method that the script `Helpers` defines and that raises on line 2 of `Helpers`
- **THEN** the report names `Helpers` and line 2

#### Scenario: Inside eval
- **WHEN** line 4 of a script evaluates `eval("raise 'in eval'")`
- **THEN** the report names that script and line 4

#### Scenario: A missing file
- **WHEN** a script evaluates `load_data("Data/Nope.rvdata2")`
- **THEN** the report is `Unable to find file:`, an empty line and `Data/Nope.rvdata2`

#### Scenario: An empty message
- **WHEN** line 1 of a script named `s` evaluates `raise RuntimeError, ""`
- **THEN** the report is `Script 's' line 1: RuntimeError occurred.` and nothing else

#### Scenario: No line
- **WHEN** a script raises an exception whose backtrace was set to an empty array, with the message `adrift`
- **THEN** the report is `RuntimeError occurred.`, an empty line and `adrift`

#### Scenario: Syntax error
- **WHEN** a script named `Bad Syntax` does not parse at its line 3
- **THEN** the report starts with `Script 'Bad Syntax' line 3: SyntaxError occurred.`, its message does not start with `{`, the scripts before it have run and no later one runs

#### Scenario: A changed name
- **WHEN** the first script assigns `$RGSS_SCRIPTS[1][1] = "Renamed"` and the second raises
- **THEN** the report names `Renamed`

#### Scenario: Exceptions that are no StandardError
- **WHEN** a script raises `Exception`, `Interrupt` or `NotImplementedError`, or lets a `throw :sym` go uncaught
- **THEN** each is reported in the same form, the last as an `ArgumentError` whose message names `:sym`

### Requirement: Reset of RGSS3
In a game of RGSS3 a reset request of the host SHALL make the next `Graphics.update` raise
`RGSSReset` with an empty message. An `RGSSReset` that leaves a script SHALL make the guest
call `Graphics.__reset__`, stop the audio, clear the input state and evaluate the preload
scripts and the game's scripts again from the first, in the same interpreter. One that a
script rescues SHALL restart nothing. The seed SHALL be set once, before the built-in
scripts.

#### Scenario: Reset by the host, measured on the original
- **WHEN** the host sends a reset request while a script loops `Graphics.update` inside `begin ... rescue RGSSReset => e`
- **THEN** the rescue branch runs and `e.message` is `""`

#### Scenario: A bare rescue does not catch it, measured on the original
- **WHEN** the loop is inside `begin ... rescue => e` instead
- **THEN** the branch does not run and the scripts start over

#### Scenario: At the top level
- **WHEN** an archive of two scripts is run whose first counts its evaluations in a global and makes a sprite, and whose second raises `RGSSReset` the first two times it runs
- **THEN** the first script was evaluated three times, globals kept their values, the sprite of each earlier run is disposed, and the run ends with success

#### Scenario: Inside the main function
- **WHEN** the block of `rgss_main` of the stand-in raises `RGSSReset` on its first run after making a sprite and setting `Graphics.frame_count = 1000`, `Graphics.brightness = 99` and `Graphics.frame_rate = 30`
- **THEN** the block runs a second time in which the sprite is disposed, `Graphics.brightness` is 255, `Graphics.frame_rate` is 60 and `Graphics.frame_count` is at least 1000, and the first script was evaluated once

#### Scenario: Random numbers across a reset
- **WHEN** a game that appends `rand(1000)` to a global array in each run of its block and raises `RGSSReset` once is run twice with seed 7
- **THEN** both runs give the same two numbers

### Requirement: Data files of RGSS3
In a game of RGSS3 `load_data(name)` SHALL return what `Marshal.load` returns, called through
Ruby with the bytes of the data file and a proc that labels every String it is handed whose
encoding is ASCII-8BIT as UTF-8. `save_data(object, name)` SHALL write the bytes of
`Marshal.dump(object)` to the file and return `nil`. A missing file SHALL raise
`Errno::ENOENT` with the name as given.

#### Scenario: Plain strings, measured on the original
- **WHEN** a data file holds the dump `04 08 5b 07 22 08 61 62 63 22 08 e6 b5 8b` (an array of two strings without an encoding) and a script maps `load_data` of it to `[s.encoding.name, s.valid_encoding?]`, and does the same with `Marshal.load` of the bytes
- **THEN** the first result is `[["UTF-8", true], ["UTF-8", true]]` and the second names `ASCII-8BIT` twice

#### Scenario: Strings inside objects, measured on the original
- **WHEN** a data file holds the dump of a Hash with a plain String as a key and an Array with a plain String as its value, written without encodings
- **THEN** the key and the string in the array are both labelled UTF-8 after `load_data`

#### Scenario: Strings that carry an encoding, measured on the original
- **WHEN** a data file holds strings with `:E` true, with `:E` false and with `:encoding` `"UTF-16LE"`
- **THEN** after `load_data` they are labelled UTF-8, US-ASCII and UTF-16LE

#### Scenario: Bytes that are not UTF-8, measured on the original
- **WHEN** a data file holds the plain string of the one byte `ff`
- **THEN** after `load_data` it is labelled UTF-8 and `valid_encoding?` is `false`

#### Scenario: A redefined Marshal.load, measured on the original
- **WHEN** a script redefines `Marshal.load` to return `:mine` and calls `load_data` of an existing file
- **THEN** the result is `:mine`

#### Scenario: Save then load, measured on the original
- **WHEN** `save_data(["abc".force_encoding("ASCII-8BIT"), Color.new(1, 2, 3, 4), {1 => 2}], "Out.rvdata2")` is evaluated and the file is read
- **THEN** the value of the call is `nil`, the bytes of the file are those of `Marshal.dump` of the same object, and `load_data("Out.rvdata2")[0].encoding.name` is `"UTF-8"`

#### Scenario: Missing, measured on the original
- **WHEN** `load_data("Data/Nope.rvdata2")` is evaluated
- **THEN** `Errno::ENOENT` is raised with the message `No such file or directory - Data/Nope.rvdata2`

#### Scenario: Data of the value classes
- **WHEN** a data file written by `save_data` holds a `Table`, a `Color`, a `Tone` and a `Rect`
- **THEN** `load_data` returns objects of those classes whose `_dump(0)` equals that of the saved ones

### Requirement: Message boxes and console output of RGSS3
In a game of RGSS3 `msgbox` and `msgbox_p` SHALL send the host a modal message, write its
text to the error stream and return `nil` once the host has dismissed it. `p`, `print` and
`puts` SHALL be those of Ruby: they SHALL write to the standard output of the program, show
no message and wait for nothing. This SHALL hold in the place of the requirement "Message
output".

#### Scenario: Text of a box, measured on the original
- **WHEN** `msgbox 1, "a", nil, :b, 1.5, [1, "x"]` is evaluated
- **THEN** the host receives a modal message with the text `1anilb1.5[1, "x"]`

#### Scenario: Inspect, measured on the original
- **WHEN** `msgbox_p "a", nil` is evaluated
- **THEN** the host receives a modal message with the text `"a"` and `nil` on separate lines

#### Scenario: Separators and NUL, measured on the original
- **WHEN** `$, = "-"; msgbox 1, 2` and then `$, = nil; msgbox "ab\0cd"` are evaluated
- **THEN** the texts are `1-2` and `ab`

#### Scenario: Value
- **WHEN** a script evaluates `[msgbox("x"), msgbox_p(1)]` against a host that dismisses each message
- **THEN** the result is `[nil, nil]`

#### Scenario: Console output, measured on the original
- **WHEN** a generated game of RGSS3 whose script evaluates `$r = p("a", 1); print "b", 2; puts "c"` and writes `$r.inspect` to a file is run headless, with and without `--log-p`
- **THEN** in both runs the standard output of the program is `"a"`, `1`, `b2c` on three lines, the file holds `["a", 1]`, no modal message was shown and the exit status is 0

### Requirement: Process functions of Ruby 1.9 are off by default
Unless the caller enables them, in a guest over Ruby 1.9.2 `spawn`, `Process.spawn`,
`Process.exec` and `Kernel#open` with a command SHALL start nothing, SHALL return `nil` and
SHALL write one line naming the call to the error stream, as the functions of the
requirement "Process functions are off by default" do, which SHALL hold over Ruby 1.9.2 too.
`open` with a file name SHALL work as before.

#### Scenario: Functions of Ruby 1.9
- **WHEN** a generated game of RGSS3 run with `--sandbox off` evaluates `spawn("touch a")`, `Process.spawn("touch b")`, `open("|touch c")`, `system("touch", "d")`, `` `touch e` `` and `IO.popen(["touch", "f"])`
- **THEN** each result is `nil`, none of the files `a` to `f` exists, and the error stream has one line for each call

#### Scenario: Open with a file name
- **WHEN** the same game evaluates `open("note.txt", "w") { |file| file.write("x") }`
- **THEN** the file `note.txt` holds `x`

#### Scenario: Enabled by the caller
- **WHEN** the game is run with `--allow-process-functions` and evaluates `system("true")` and `Process.wait(spawn("true"))`
- **THEN** the first result is `true` and the second is a process number

#### Scenario: Games of RGSS1 and RGSS2
- **WHEN** the tests of the requirement "Process functions are off by default" run over Ruby 1.8.7
- **THEN** they pass unedited

### Requirement: Version of RGSS3 by the library of the game
In a game of RGSS3 `RGSS_VERSION` SHALL be `"3.0.0"` when the DLL the game's `Library`
names has a version whose first three numbers are 3.0.0, and `"3.0.1"` for any other
version, a DLL without a version record and no DLL, in ASCII-8BIT and defined once. This
SHALL hold in the place of the version that the requirement "Preparation of a game of
RGSS3" states.

#### Scenario: Versions, measured on the original
- **WHEN** a game of RGSS3 is run with the library versions 3.0.0.1 and 3.0.1.1 and evaluates `[RGSS_VERSION, RGSS_VERSION.encoding.name]`
- **THEN** the results are `["3.0.0", "ASCII-8BIT"]` and `["3.0.1", "ASCII-8BIT"]`

#### Scenario: The table of the decision
- **WHEN** a game of RGSS3 is run with the library version 3.0.0.7, with 2.0.2.1 and without a version
- **THEN** `RGSS_VERSION` is `"3.0.0"`, `"3.0.1"` and `"3.0.1"`, and no warning of a constant defined twice is written

#### Scenario: Through both executables
- **WHEN** a generated game of RGSS3 whose `Library=System\RGSS300.dll` is a file holding a version record is run headless by `vkrgss`, once with 3.0.0.1 and once with 3.0.1.1, and reports `RGSS_VERSION`
- **THEN** it reports `"3.0.0"` and `"3.0.1"`

#### Scenario: Before this change
- **WHEN** the tests of the preparation of RGSS3 of `run-rgss3-games-in-a-guest-of-their-own` run with their configuration, which names no library version
- **THEN** they pass unedited, `RGSS_VERSION` being `"3.0.1"`

### Requirement: Built-in scripts of RGSS3 are bytes
In a game of RGSS3 the built-in scripts file SHALL be evaluated from a string of bytes, so
that a string literal of it is ASCII-8BIT, while a literal of a game script SHALL be UTF-8.

#### Scenario: Two literals
- **WHEN** a built-in scripts file assigns `''` to a global and a game script of RGSS3 does the same to another
- **THEN** the first string's encoding is ASCII-8BIT and the second's UTF-8

#### Scenario: A dump
- **WHEN** the built-in scripts file dumps `['', 'a']` with `Marshal.dump`
- **THEN** the dump is `04085b072200220661` in hexadecimal: neither string carries an encoding

### Requirement: Win32API of RGSS3
In a game of RGSS3 `Win32API` SHALL have the methods `initialize` (arity 4, private), `call`
and `Call` (arity -1) of the original runtime, SHALL take the library and the function as
Strings and the declarations as a String, an Array of Strings or `nil`, SHALL keep its state
out of `instance_variables`, and SHALL not be dumped. This SHALL hold in the place of the
requirement "Win32API placeholder" where the two differ.

#### Scenario: Method list, measured on the original
- **WHEN** the test of the method list of RGSS3 checks `Win32API`
- **THEN** its three methods have the arities of the list and it has no other public method of its own

#### Scenario: Arguments, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "i")`, `Win32API.new(:user32, "GetKeyState", "i", "i")` and `Win32API.new("user32", "GetKeyState", nil, nil)`
- **THEN** the first raises `ArgumentError` with `wrong number of arguments(3 for 4)`, the second `TypeError` with `can't convert Symbol into String`, and the third returns a `Win32API`

#### Scenario: The object, measured on the original
- **WHEN** a script evaluates `api = Win32API.new("user32", "GetKeyState", "i", "i")`, then `[api.instance_variables, api.dup.call(0x41), Class.new(Win32API).new("user32", "GetKeyState", "i", "i").call(0x41)]` and `Marshal.dump(api)`
- **THEN** the first is `[[], 0, 0]` and the dump raises `TypeError` with `no marshal_dump is defined for class Data`

### Requirement: Libraries of Win32API in RGSS3
In a game of RGSS3 `Win32API.new` SHALL find a library that is a file of the game, the
runtime library of `Game.ini`, or a Windows system library of the list of this change's
design, without regard to letter case and with or without `.dll`, and SHALL raise
`RuntimeError` with `LoadLibrary: <name>` and a line feed for any other. A function of a
found library outside the emulation table SHALL be reported once per pair, as before.

#### Scenario: A missing library, measured on the original
- **WHEN** a script evaluates `Win32API.new("nosuch", "Function", "", "")`
- **THEN** it raises `RuntimeError` with the message `"LoadLibrary: nosuch\n"`

#### Scenario: The game's own libraries, measured on the original
- **WHEN** a game whose `Game.ini` names `System\RGSS301.dll` and whose directory holds that file and `tktk_bitmap.dll` evaluates `Win32API.new` with `"System/RGSS301.dll"`, `"RGSS301"` and `"tktk_bitmap"`, then `call` of each
- **THEN** no constructor raises, each call returns 0, and the error stream has one line for each library and function pair

#### Scenario: System libraries
- **WHEN** a script makes `Win32API` objects of `"wininet"`, `"Kernel32"`, `"USER32.DLL"` and `"msvcrt"`
- **THEN** none raises

### Requirement: Declarations and count of Win32API in RGSS3
In a game of RGSS3 each character `p`, `l`, `i` or `n` of the import, in either case, SHALL
declare one parameter, `v` and every other character none; an Array of Strings SHALL
declare the first character of each. A call with another number of arguments SHALL raise
`RuntimeError` with `wrong number of parameters: expected <declared>, got <given>`.

#### Scenario: Ignored characters, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "pointer", "i").call(1, 2)`
- **THEN** it raises `RuntimeError` with `wrong number of parameters: expected 3, got 2`

#### Scenario: Count, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "i", "i").call`
- **THEN** it raises `RuntimeError` with `wrong number of parameters: expected 1, got 0`

#### Scenario: Array, spaces and v
- **WHEN** a script calls `Win32API` objects declared `["l", "p", "p", "l", "l", "p"]` with six arguments, `"n i i i i"` with five, `[]` and `"v"` with none
- **THEN** none raises

### Requirement: Parameters of Win32API in RGSS3
In a game of RGSS3 an integer parameter SHALL be converted as the interpreter converts to an
unsigned long and taken by its low 32 bits; a pointer parameter SHALL take a String that is
not frozen as a buffer, `nil` and 0 as NULL, another Integer as an address, and SHALL raise
`TypeError` for anything else. The errors SHALL be worded as measured on the original.

#### Scenario: Integer parameters, measured on the original
- **WHEN** a script calls `GetKeyState` declared `"i"` with `nil`, `"a"`, `true` and `65.9`
- **THEN** the first raises `TypeError` with `no implicit conversion from nil to integer`, the second `TypeError` with `can't convert String into Integer`, the third `TypeError`, and the last returns what `GetKeyState` gives for key 65

#### Scenario: Pointer parameters, measured on the original
- **WHEN** a script calls `GetKeyboardState` declared `"p"` with a frozen String of 256 bytes, `1.5`, `:sym` and `[]`
- **THEN** the frozen String raises `RuntimeError` with `can't modify frozen string`, and `1.5`, `:sym` and `[]` raise `TypeError` with `can't convert Float into String`, `can't convert Symbol into String` and `can't convert Array into String`

#### Scenario: Object ids
- **WHEN** a script calls a function of `tktk_bitmap` declared `"n"` with `Bitmap.new(1, 1).object_id`
- **THEN** it returns 0 and raises nothing

#### Scenario: Unsigned, measured on the original
- **WHEN** a script calls `GetKeyState` declared `"i"` with `2**32 - 1` and with `2**64`
- **THEN** the first returns 0, and the second raises `RangeError` with ``bignum too big to convert into `unsigned long'``

### Requirement: Values of Win32API in RGSS3
In a game of RGSS3 the export `v` SHALL give 0, the export `p` SHALL raise `ArgumentError`
with `NULL pointer given` for 0 and give an empty ASCII-8BIT String otherwise, and any other
export SHALL give a signed 32-bit Integer. A buffer an emulated function fills SHALL keep
its length and its encoding label.

#### Scenario: Exports, measured on the original
- **WHEN** a script calls `GetSystemMetrics` with 5 declared with the exports `"v"`, `"p"`, `""`, `nil` and `"x"`
- **THEN** the first gives 0, the second raises `ArgumentError` with `NULL pointer given`, and the others give 0

#### Scenario: Signed, measured on the original
- **WHEN** `GetKeyState` declared `"i"` reports a held key
- **THEN** the value is negative, as a signed 32-bit Integer

#### Scenario: Not emulated with p
- **WHEN** a script calls a function of `user32` outside the emulation table declared with the export `"p"`
- **THEN** the value is an empty ASCII-8BIT String and one line names the pair

#### Scenario: A buffer keeps its label, measured on the original
- **WHEN** a script passes the UTF-8 String `"\0" * 16` to `GetClientRect` with the handle of `FindWindowA`
- **THEN** after the call the String is still labelled UTF-8 and has 16 bytes, which unpack with `"l4"` to `[0, 0, 544, 416]`

### Requirement: Measured calls of Win32API in RGSS3
In a game of RGSS3 the profile functions SHALL read nothing and write nothing for a file
name without `/` or `\`, the window class `RGSS Player` SHALL find the game window, and an
Integer source of `RtlMoveMemory` SHALL copy nothing. The other emulated functions SHALL
give the values measured on the original runtime where they were measured.

#### Scenario: Title, measured on the original
- **WHEN** a game whose `Game.ini` has a title of 16 bytes calls `GetPrivateProfileStringA` declared `"pppplp"` with `("Game", "Title", "", buf, 255, "./Game.ini")` and then with the file `"Game.ini"`
- **THEN** the first returns 16 with the title in the buffer, and the second returns 0

#### Scenario: Window, measured on the original
- **WHEN** a script calls `FindWindowA` with `"RGSS Player"` and the title, and `GetClientRect` with the handle and a 16-byte buffer
- **THEN** the handle is not 0 and the rectangle is 0, 0, 544, 416

#### Scenario: Cursor, measured on the original
- **WHEN** the first call of `ShowCursor` of a run is `ShowCursor(0)`
- **THEN** it returns -1

#### Scenario: Text, measured on the original
- **WHEN** `MultiByteToWideChar(65001, 0, s, -1, buf, n)` converts the UTF-8 bytes of `日本` and `WideCharToMultiByte` converts the result to 65001 and to 932
- **THEN** the bytes are those of `日本` in UTF-8 and in Shift_JIS, each with a zero byte

#### Scenario: The address trick
- **WHEN** a script calls `RtlMoveMemory` declared `"pll"` with a buffer of 4 bytes, `bitmap.object_id * 2 + 16` and 4
- **THEN** the buffer is unchanged and nothing raises

### Requirement: DL of RGSS3
In a game of RGSS3 the module `DL` SHALL exist with the numeric constants of the original's
method list and the classes `DLError`, `DLTypeError`, `Handle`, `CFunc` and `CPtr`, and
every function of the module and every constructor and class method of those three classes
SHALL raise `DL::DLError` with a message that names the call. Games of RGSS1 and RGSS2
SHALL have no `DL`.

#### Scenario: Constants, measured on the original
- **WHEN** a script evaluates `[DL::TYPE_VOID, DL::TYPE_INT, DL::TYPE_DOUBLE, DL::SIZEOF_VOIDP, DL::SIZEOF_LONG_LONG, DL::ALIGN_SHORT, DL::RTLD_NOW, DL::DLError.superclass, DL::DLTypeError.superclass]`
- **THEN** the result is `[0, 4, 8, 4, 8, 2, -1, StandardError, DL::DLError]`

#### Scenario: Refused
- **WHEN** a script evaluates `DL.dlopen("user32")`, `DL::Handle.new("user32")`, `DL::CFunc.new(0)`, `DL::CPtr.malloc(8)` and `DL.malloc(8)`
- **THEN** each raises `DL::DLError` whose message names the call

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 or RGSS2 evaluates `defined?(DL)`
- **THEN** the result is `nil`

### Requirement: Win32API of XP and VX over Ruby 1.9.2
The tests of the emulated `Win32API` functions of RGSS1 and RGSS2 SHALL pass over both
interpreters with their expected values unedited, without an entry of the allow-list of
Ruby 1.9.2, and `Win32API` of RGSS1 and RGSS2 SHALL keep its behaviour.

#### Scenario: Keyboard state
- **WHEN** the tests of the toggle, of `SetKeyboardState` and of `GetKeyboardState` run over Ruby 1.9.2
- **THEN** they pass, and the allow-list has no entry for `win32api`

#### Scenario: Unchanged
- **WHEN** the existing tests of `tests/win32api.rs` run over Ruby 1.8.7
- **THEN** they pass unedited but for the Ruby snippets of the three tests above

### Requirement: Program name of a game of RGSS3
In a game of RGSS3 `$0` and `$PROGRAM_NAME` SHALL be `false`, and the last entry of the
backtrace of a script SHALL be the frame of the `eval` that runs it, named
``ruby:in `eval'``, as measured on the original runtime. Games of RGSS1 and RGSS2 SHALL keep
what they have.

#### Scenario: Globals, measured on the original
- **WHEN** the first script of a generated game of RGSS3 evaluates `[$0, $PROGRAM_NAME]`
- **THEN** the result is `[false, false]`

#### Scenario: The last frame, measured on the original
- **WHEN** line 3 of the script at index 2 evaluates `caller(0)`
- **THEN** its first entry starts with `{0002}:3:` and its last entry is ``ruby:in `eval'``

#### Scenario: XP and VX
- **WHEN** the existing tests of the preparation of games of RGSS1 and RGSS2 run over both interpreters
- **THEN** they pass unedited

### Requirement: Long reports and the status after an error in RGSS3
In a game of RGSS3 the report of an uncaught exception SHALL be sent and written whole
whatever its length, and the run SHALL end with status 1 whatever the line of the error.
These are vkrgss's rules where the original shows an empty box above 1023 UTF-16 units and
ends with a code that follows the line in most cases.

#### Scenario: A long message
- **WHEN** line 1 of a script named `s` raises a `RuntimeError` whose message is 2000 times `x`
- **THEN** the modal message is `Script 's' line 1: RuntimeError occurred.`, an empty line and the 2000 characters, and the same text is on the error stream

#### Scenario: Status whatever the line
- **WHEN** generated games of RGSS3 are run headless whose script raises on line 3, whose script raises `Errno::ENOENT`, and whose script raises with an empty backtrace
- **THEN** each run exits with status 1

### Requirement: A script archive of RGSS3 that cannot be loaded
When the script archive of a game of RGSS3 cannot be loaded, the guest SHALL show the modal
message `Failed to load script.` and write it to the error stream before the outcome and
the shutdown message; the error that names the archive SHALL stay on the error stream, and
the run SHALL end with status 1. An archive that holds an empty Array SHALL run nothing and
end with status 0.

#### Scenario: The cases, measured on the original
- **WHEN** a game of RGSS3 is run whose archive is missing, empty, not a `Marshal` dump, a dump of a String, or an Array whose entry has no third value or a third value that is not deflated data
- **THEN** in each run the host receives the modal message `Failed to load script.`, no script has run, and the outcome is a failure

#### Scenario: Through the program
- **WHEN** a generated game of RGSS3 whose archive is not a `Marshal` dump is run headless
- **THEN** the error stream has `Failed to load script.` and a line that names the archive, and the exit status is 1

#### Scenario: Empty archive, measured on the original
- **WHEN** a game of RGSS3 whose archive is an empty Array is run headless
- **THEN** no modal message is shown and the exit status is 0

### Requirement: at_exit and exit! in RGSS3
In a game of RGSS3 `at_exit` SHALL return its block as a Proc and never run it, also through
`Kernel.at_exit`. `exit!(status = false)` and `Process.exit!` SHALL end the game at once,
running no `ensure` clause, after the guest stopped the audio and sent the outcome and the
shutdown message; the program SHALL exit with status 0 for `true` or 0 and with 1 otherwise.

#### Scenario: Value of at_exit
- **WHEN** a script evaluates `[at_exit { 1 }.class, Kernel.at_exit { 2 }.class]`
- **THEN** the result is `[Proc, Proc]`

#### Scenario: Never run, measured on the original
- **WHEN** generated games of RGSS3 whose first script registers `at_exit { File.open("at_exit.txt", "w") { } }` end by the end of their last script, by `exit(3)`, by an uncaught exception and by `exit!(0)`, each run headless
- **THEN** no run leaves a file `at_exit.txt`

#### Scenario: exit!, measured on the original
- **WHEN** generated games of RGSS3 evaluate `begin; exit!(0); ensure; File.open("ensure.txt", "w") { }; end`, `exit!(4)`, `exit!(true)`, `exit!` and `Process.exit!(0)`, each run headless
- **THEN** the exit statuses are 0, 1, 0, 1 and 0, no run leaves a file `ensure.txt`, and no later script has run

### Requirement: Magic comments of the scripts of RGSS3
A magic comment on the first line of a script of RGSS3 SHALL set the encoding of the script
as measured on the original for the names UTF-8, ASCII-8BIT, US-ASCII and UTF-16LE and for
names the interpreter does not know, and a coding comment below the first line after a
comment SHALL be ignored.

#### Scenario: ASCII-8BIT, measured on the original
- **WHEN** scripts whose first line is `# coding: ascii-8bit`, `# encoding: ASCII-8BIT` or `# vim: set fileencoding=ascii-8bit :` evaluate `[__ENCODING__.name, "日本".size]`
- **THEN** each result is `["ASCII-8BIT", 6]`

#### Scenario: UTF-8 and the second line, measured on the original
- **WHEN** a script without a comment, one whose first line is `# coding: utf-8`, and one whose second line is `# coding: ascii-8bit` after a comment line evaluate `__ENCODING__.name`
- **THEN** each result is `"UTF-8"`

#### Scenario: US-ASCII, measured on the original
- **WHEN** a script named `magic` whose first line is `# coding: us-ascii` has a literal with a byte above 127 on line 2
- **THEN** the report is `Script 'magic' line 2: SyntaxError occurred.`, an empty line and `invalid multibyte char (US-ASCII)`

#### Scenario: Unknown names, measured on the original
- **WHEN** generated games of RGSS3 are run headless whose script's first line is `# coding: euc-jp`, `# coding: binary` or `# coding: nosuch`
- **THEN** each report is `ArgumentError occurred.`, an empty line and `unknown encoding name: ` with the name as written, and each exit status is 1

#### Scenario: UTF-16LE, measured on the original
- **WHEN** a script's first line is `# coding: utf-16le`
- **THEN** the report is `ArgumentError occurred.`, an empty line and `UTF-16LE is not ASCII compatible`

### Requirement: Source and errors of load_data in RGSS3
In a game of RGSS3 `load_data` SHALL take one argument and call `Marshal.load` through Ruby
with an open `File` for a loose file and a String of the bytes for an archive entry, and a
proc that labels each ASCII-8BIT String UTF-8, a frozen one too. A directory SHALL raise
`Errno::EACCES`. This SHALL hold in the place of the requirement "Data files of RGSS3" where
the two differ.

#### Scenario: A frozen string, measured on the original
- **WHEN** a data file holds an object of a user class whose `_load` returns `"ret".force_encoding("ASCII-8BIT").freeze`, and a script evaluates `[load_data(f).encoding.name, load_data(f).frozen?]`
- **THEN** the result is `["UTF-8", true]`

#### Scenario: Cut loose files, measured on the original
- **WHEN** `load_data` reads an empty loose file, a loose file of the byte `04`, and a loose file holding the dump of `[1, 2, 3]` without its last byte
- **THEN** each raises `EOFError` with the message `end of file reached`

#### Scenario: A string cut short, measured on the original
- **WHEN** `load_data` reads a loose file holding the dump of `"abcdef"` without its last two bytes
- **THEN** it raises `ArgumentError` with the message `marshal data too short`

#### Scenario: Not a dump, measured on the original
- **WHEN** `load_data` reads a loose file that starts with the bytes of `[Game]`
- **THEN** it raises `TypeError` whose message starts with `incompatible marshal file format (can't be read)`

#### Scenario: A directory, measured on the original
- **WHEN** `load_data("d")` is evaluated and `d` is a directory of the game
- **THEN** it raises `Errno::EACCES` with the message `Permission denied - d`

#### Scenario: Arguments, measured on the original
- **WHEN** `load_data`, `load_data("a", "b")`, `load_data(nil)` and `load_data(:sym)` are evaluated
- **THEN** they raise `ArgumentError` with `wrong number of arguments(0 for 1)` and `(2 for 1)`, and `TypeError` with `can't convert nil into String` and `can't convert Symbol into String`

#### Scenario: An object with to_str, measured on the original
- **WHEN** `load_data` is given an object whose `to_str` returns the name of an existing data file
- **THEN** it returns the object of that file

#### Scenario: Only the first object, measured on the original
- **WHEN** a loose file holds the dump of `1` followed by the dump of `2`
- **THEN** `load_data` of it returns `1`

### Requirement: Names of data files in RGSS3
In a game of RGSS3 `load_data` and `save_data` SHALL raise `ArgumentError` for a name with a
NUL byte and `Encoding::UndefinedConversionError` for a name labelled ASCII-8BIT whose bytes
are not valid UTF-8, naming the first such byte, before any file is looked up or written; a
name of valid UTF-8 SHALL name the file of those characters, as the original runtime 3.0.1
does.

#### Scenario: NUL, measured on the original 3.0.1
- **WHEN** `load_data("Game.ini\0x")` is evaluated
- **THEN** it raises `ArgumentError` with the message `string contains null byte`

#### Scenario: Bytes that are not UTF-8, measured on the original 3.0.1
- **WHEN** `save_data("sj", "d/\x83\x65.bin".force_encoding("ASCII-8BIT"))` and `load_data` of the same name are evaluated
- **THEN** each raises `Encoding::UndefinedConversionError` with the message `"\x83" from ASCII-8BIT to UTF-8`, and no file was written

#### Scenario: UTF-16LE, measured on the original 3.0.1
- **WHEN** `load_data("d/p.bin".encode("UTF-16LE"))` is evaluated
- **THEN** it raises `ArgumentError` with the message `string contains null byte`

#### Scenario: Invalid bytes labelled UTF-8, measured on the original 3.0.1
- **WHEN** `save_data("bad", "d/\xFF\xFE.bin".force_encoding("UTF-8"))` is evaluated
- **THEN** the value is `nil` and a file was written into `d`

#### Scenario: A name of other characters, measured on the original 3.0.1
- **WHEN** `save_data([1], "測試.rvdata2")` is evaluated and then `load_data` of the same name, once labelled UTF-8 and once labelled ASCII-8BIT
- **THEN** both return `[1]`, and the game directory holds a file of that UTF-8 name

### Requirement: Loading code in RGSS3
In a game of RGSS3 `require` and `load` SHALL behave as the requirement "Loading code" says,
and `require_relative` and `autoload` SHALL load only what `require` may: a `.rb` file inside
the game directory. A file that `require`, `load` or `require_relative` loads SHALL be
compiled as UTF-8; a file that `autoload` loads is compiled by the interpreter's own loader.

#### Scenario: require
- **WHEN** the game directory holds `Extra.rb` and a script evaluates `[require("./Extra.rb"), require("Extra"), $LOADED_FEATURES.size, (require("nosuch") rescue $!.message), (require("zlib") rescue $!.class)]`
- **THEN** the result is `[true, false, 2, "no such file to load -- nosuch", LoadError]` and the second entry of `$LOADED_FEATURES` ends with `Extra.rb`

#### Scenario: require_relative
- **WHEN** a script evaluates `require_relative "lib/A"` and `lib/A.rb` of the game directory evaluates `require_relative "B"`
- **THEN** both `lib/A.rb` and `lib/B.rb` were evaluated once, and `require_relative "../outside"` raises `LoadError`

#### Scenario: autoload
- **WHEN** the game directory holds `Lazy.rb`, which defines `Lazy` and counts its evaluations, and a script evaluates `autoload(:Lazy, "Lazy")`, reads the count, uses `Lazy`, and evaluates `require "Lazy"`
- **THEN** the count is 0 before the use and 1 after it, and `require` returns `false`

#### Scenario: autoload outside the game directory
- **WHEN** a script evaluates `autoload(:Far, "/etc/passwd")` and `Object.autoload(:Gone, "nosuch")`
- **THEN** both raise `LoadError` at the call, and the second message is `no such file to load -- nosuch`

#### Scenario: Compiled as UTF-8
- **WHEN** a required file holds `$vkrgss_enc = ["測".encoding.name, "測".size]`
- **THEN** `$vkrgss_enc` is `["UTF-8", 1]`
