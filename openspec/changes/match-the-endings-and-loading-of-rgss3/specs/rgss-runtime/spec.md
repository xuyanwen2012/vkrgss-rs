## ADDED Requirements

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
These are vkxp's rules where the original shows an empty box above 1023 UTF-16 units and
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
the game directory. A file they load SHALL be compiled as UTF-8.

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
- **WHEN** a required file holds `$vkxp_enc = ["測".encoding.name, "測".size]`
- **THEN** `$vkxp_enc` is `["UTF-8", 1]`
