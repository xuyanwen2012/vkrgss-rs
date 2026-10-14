## MODIFIED Requirements

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
