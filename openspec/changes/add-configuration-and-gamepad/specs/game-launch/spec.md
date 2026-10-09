# Spec Delta

## ADDED Requirements

### Requirement: Configuration files
The program SHALL read settings from a user configuration file and from `vkxp.toml` in the game
directory when they exist, with the game's file overriding the user's and command-line options
overriding both. Every setting that has a command-line option SHALL be settable in the files.
`--config FILE` SHALL name the user file and `--no-config` SHALL make the program read neither.

#### Scenario: RTP from the file
- **WHEN** the user file names the directory of the RTP `Standard` and a game that needs it is run without `--rtp`
- **THEN** the game finds its RTP files

#### Scenario: Precedence
- **WHEN** the user file sets seed 1, the game's file sets seed 2 and the command line gives no seed
- **THEN** the run uses seed 2, and with `--seed 3` it uses seed 3

#### Scenario: Ignored
- **WHEN** the program is run with `--no-config` and a user file with an error exists
- **THEN** the run starts normally

### Requirement: Configuration errors
A configuration file that does not parse, or that holds an unknown key or a value of the wrong
type, SHALL end the program with an error naming the file and the key before any window is
opened.

#### Scenario: Unknown key
- **WHEN** a configuration file holds a key the program does not know
- **THEN** the program exits with a non-zero status and names the file and the key
