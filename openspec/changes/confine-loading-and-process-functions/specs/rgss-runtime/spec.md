## MODIFIED Requirements

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
