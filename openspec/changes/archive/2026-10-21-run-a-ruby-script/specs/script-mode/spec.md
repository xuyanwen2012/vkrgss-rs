## ADDED Requirements

### Requirement: Run a Ruby file without a game
`vkrgss --script FILE [ARG...]` SHALL run the Ruby file `FILE` with the interpreter of the
runtime that `--runtime` names (Ruby 1.8.7 for `rgss1` and `rgss2`, Ruby 1.9.2 for `rgss3`,
the default), with the arguments after `FILE` as `ARGV`, and SHALL NOT open a window, a Vulkan
device or an audio device. The modules and classes of RGSS SHALL NOT be defined.

#### Scenario: Output and arguments
- **WHEN** `vkrgss --script s.rb a b` runs a file that prints `ARGV.inspect` and `RUBY_VERSION`
- **THEN** the standard output holds `["a", "b"]` and `1.9.2`, and the exit status is 0

#### Scenario: The interpreter of RGSS1
- **WHEN** the same file runs with `--runtime rgss1`
- **THEN** the standard output holds `1.8.7`

#### Scenario: No RGSS
- **WHEN** a script refers to `Bitmap`
- **THEN** it raises `NameError`

### Requirement: Streams and exit status of a script
A script SHALL read the standard input of the process and write to its standard output and
error streams, and the exit status SHALL be that of `exit`, 0 at the end of the file, and 1
after an uncaught exception, whose message SHALL be written to the error stream.

#### Scenario: Standard input
- **WHEN** a script that prints the sum of the numbers on its input is given `1 2 3`
- **THEN** it prints `6`

#### Scenario: Exit status
- **WHEN** a script calls `exit 3`
- **THEN** the exit status is 3

#### Scenario: Uncaught exception
- **WHEN** a script raises `RuntimeError` with the message `boom`
- **THEN** the exit status is 1 and the error stream holds `boom`

### Requirement: Loading files from a script
`require`, `load`, `require_relative` and `autoload` in a script SHALL follow the rules they
have in a game, with the script's directory in place of the game directory.

#### Scenario: Require beside the script
- **WHEN** a script requires `helper` and `helper.rb` lies beside it
- **THEN** the file is loaded

#### Scenario: No standard library
- **WHEN** a script requires `set`
- **THEN** it raises `LoadError`

### Requirement: Confinement of a script
On Linux a script SHALL be confined as a guest process is, with its own directory as the
directory it may write below, unless `--sandbox off` is given; starting a program SHALL be
refused unless `--allow-process-functions` is given.

#### Scenario: Writes outside the script's directory
- **WHEN** a script writes a file outside its directory
- **THEN** the write fails, and a write beside the script succeeds

#### Scenario: No program is started
- **WHEN** a script calls `system("true")` without `--allow-process-functions`
- **THEN** no program runs
