## ADDED Requirements

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
- **THEN** it says in a comment that it was written for the tests of vkxp, and it is the text given in this change's `design.md`

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
