## ADDED Requirements

### Requirement: Record of a guest process
The program SHALL hand a guest process what it needs of the run (the game directory, the
settings a guest reads, the seed, the socket, the diagnostic level) in a record: a file it
writes into the private directory of the socket before it starts the process, and names as
the only value of the hidden option `--guest-role`. A guest process SHALL NOT read the
options of the program's command line or a configuration file.

#### Scenario: Arguments of the guest process
- **WHEN** the program prepares the command that starts a guest process
- **THEN** the arguments of the command are `--guest-role` and the path of a file in the directory of the socket, and no other

#### Scenario: Configuration files are read once
- **WHEN** a game is run headless with `--guest process --verbose` and a user configuration file, and without a configuration file of the game
- **THEN** the run exits with status 0 and its error stream says exactly once that a configuration file was read

#### Scenario: Every setting
- **WHEN** a record with a value for every setting, two run-time packages, two font directories, two preload scripts and a path that is not UTF-8 is written and then read by the same version
- **THEN** what is read equals what was written

#### Scenario: Settings of the run
- **WHEN** the game that loads a picture of a run-time package and the game that a preload script named by a relative path patches are each run headless with `--guest process`
- **THEN** each run exits with status 0 and shows what it shows with the guest on a thread: the picture is loaded, and the dumped frame has the patch

#### Scenario: Level of the guest process
- **WHEN** a game is run headless with `--guest process --allow-process-functions --verbose` and without `VKXP_LOG`
- **THEN** the error stream has the informational line in which the guest process says that it is not confined

### Requirement: Refused guest record
A record SHALL start with a line that states the version of the program that wrote it. An
executable whose first argument is `--guest-role` SHALL end with a non-zero status and a
message on the error stream that names the file, before it opens a game or a connection,
when the record cannot be read, is not a record or is of another version than its own; for
another version the message SHALL name both. Elsewhere on the command line the option
SHALL be an unknown option.

#### Scenario: No such file
- **WHEN** `vkxp --guest-role <path>` is run with a path at which no file is
- **THEN** it exits with a non-zero status and its error stream names the path

#### Scenario: The value of version 0.2.0
- **WHEN** `vkxp --guest-role 7=/tmp/guest.sock` is run by hand
- **THEN** it exits with a non-zero status and its error stream names `7=/tmp/guest.sock` as a record that cannot be read

#### Scenario: Other version
- **WHEN** `vkxp --guest-role <file>` is run with a record that states the version `0.0.0`
- **THEN** it exits with a non-zero status and its error stream contains `0.0.0` and the version of the program

#### Scenario: Not a record
- **WHEN** `vkxp --guest-role <file>` is run with a file that holds the line of the program's version followed by bytes that are no record
- **THEN** it exits with a non-zero status and its error stream names the file

#### Scenario: Among other arguments
- **WHEN** `vkxp <game directory> --guest-role <file>` is run
- **THEN** the exit status is 2 and the error stream names `--guest-role` as an unknown option
