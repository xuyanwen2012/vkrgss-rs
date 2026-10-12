## ADDED Requirements

### Requirement: Confined guest process
A guest process SHALL restrict itself, before the first script runs, so that it can write,
create, remove and rename only below the game directory, can execute no file and, where the
kernel has the rule, can open no TCP connection; reading SHALL stay unrestricted. Where the
kernel cannot restrict, the run SHALL say so once and go on. The host process SHALL NOT be
restricted.

#### Scenario: Write inside
- **WHEN** a script of a game run with `--guest process` writes a file below the game directory
- **THEN** the file is written

#### Scenario: Write outside
- **WHEN** a script of that game opens a file for writing in a directory beside the game directory
- **THEN** `Errno::EACCES` is raised and no file is there

#### Scenario: States
- **WHEN** a headless run with `--guest process` and `--states "40:save 0, 80:load 0"` is made with and without the restriction
- **THEN** both runs dump the same frames

### Requirement: Sandbox option
`--sandbox on|off`, the environment variable `VKXP_SANDBOX` and the configuration key
`sandbox` SHALL choose whether a guest process is confined, with the command line before
the environment and the environment before the files; the default SHALL be on. With
`--allow-process-functions`, and for a guest on a thread, nothing SHALL be confined, and
the run SHALL say so once.

#### Scenario: Off
- **WHEN** the game of "Write outside" is run with `--sandbox off`
- **THEN** the file is written

#### Scenario: Unknown value
- **WHEN** the program is started with `--sandbox maybe`
- **THEN** it ends with an error that names the two values
