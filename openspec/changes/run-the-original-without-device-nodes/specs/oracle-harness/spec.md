## ADDED Requirements

### Requirement: The original side has no GPU device and no network
Every Wine process that the oracle starts SHALL run inside a `bwrap` sandbox in which
`/dev/dri` is an empty directory and the network is unshared, and SHALL be given VA-API
variables that name no driver. The oracle SHALL NOT start a Wine process outside such a
sandbox.

#### Scenario: The command that is built
- **WHEN** the oracle builds the command of any of its Wine programs
- **THEN** the program of the command is `bwrap`, its arguments hold `--tmpfs /dev/dri`, `--unshare-net` and `--die-with-parent` before the Wine program, and they do not hold `--unshare-pid`

#### Scenario: VA-API
- **WHEN** the environment of such a command is read
- **THEN** `LIBVA_DRIVERS_PATH` names a directory that does not exist and `LIBVA_DRIVER_NAME` is set

#### Scenario: bwrap is missing
- **WHEN** the original side is asked for and `bwrap` is not on `PATH`
- **THEN** the command ends with an error that names `bwrap`, before any Wine process starts
