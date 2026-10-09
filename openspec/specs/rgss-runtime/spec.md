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
