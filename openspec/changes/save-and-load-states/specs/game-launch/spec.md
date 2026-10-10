# Spec Delta

## MODIFIED Requirements

### Requirement: Guest option
The form SHALL be chosen by `--guest thread` or `--guest process`, by the environment variable
`VKXP_GUEST` and by the configuration key `guest`, the command line before the environment and
the environment before the configuration files. Without a choice the guest SHALL be a process
in a window and a thread in a headless run.

#### Scenario: Unknown value
- **WHEN** `vkxp` is started with `--guest fiber`
- **THEN** it exits with an error that names `thread` and `process`

#### Scenario: Environment
- **WHEN** `vkxp` is started with `VKXP_GUEST=process` and without `--guest`
- **THEN** the guest runs in a child process

#### Scenario: Headless default
- **WHEN** `vkxp` is started headless without a choice
- **THEN** the guest runs as a thread

## ADDED Requirements

### Requirement: States option and parked copies
`--states SCHEDULE` SHALL give a headless run its state requests, as `frame:save N` and
`frame:load N` entries separated by commas, and SHALL be an error with the guest as a thread.
States SHALL be kept as copies of the guest process, and when `vkxp` ends no process of the
run SHALL be left. The exit status SHALL follow the rules of a run without states.

#### Scenario: Thread form
- **WHEN** `vkxp` is started headless with `--states "5:save 0"` and without `--guest process`
- **THEN** it exits with an error that names `--guest process`

#### Scenario: Malformed schedule
- **WHEN** `--states "5:keep 0"` or a slot above 9 is given
- **THEN** `vkxp` exits with an error that names the entry

#### Scenario: Nothing left behind
- **WHEN** a headless run that saves into three slots, saves again into one of them and loads once ends
- **THEN** none of the processes that attached during the run exists afterwards

#### Scenario: Script error after a load
- **WHEN** the game raises a script error some frames after a state was loaded
- **THEN** the error is reported and the exit status is not 0
