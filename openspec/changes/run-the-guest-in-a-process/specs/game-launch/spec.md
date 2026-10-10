# Spec Delta

## ADDED Requirements

### Requirement: Guest in a process
`vkxp` SHALL run the guest either as a thread of its own process or in a child process. In a
child process the guest SHALL exchange with the host the messages of the protocol and nothing
else, over a socket that other users of the machine cannot reach. A run SHALL write the same
frames in both forms, and its exit status SHALL follow the same rules.

#### Scenario: Same frames
- **WHEN** a generated game that draws, moves a sprite with scripted input and makes a transition is run headless once with `--guest thread` and once with `--guest process`, dumping the same frames
- **THEN** both runs exit with status 0 and the dumped files of the two runs are byte for byte equal

#### Scenario: Script error
- **WHEN** a game whose script raises an error is run headless with `--guest process`
- **THEN** the error stream names the error as it does with `--guest thread` and the exit status is not 0

#### Scenario: Guest process dies
- **WHEN** the guest process of a headless run is killed by a signal while the game runs
- **THEN** `vkxp` ends with a status other than 0 and its error stream names the signal

#### Scenario: Frame limit
- **WHEN** a game that never ends is run headless with `--guest process --frames 5`
- **THEN** `vkxp` exits with status 0 and no guest process is left running

### Requirement: Guest option
The form SHALL be chosen by `--guest thread` or `--guest process`, by the environment variable
`VKXP_GUEST` and by the configuration key `guest`, the command line before the environment and
the environment before the configuration files. The default SHALL be `thread`.

#### Scenario: Unknown value
- **WHEN** `vkxp` is started with `--guest fiber`
- **THEN** it exits with an error that names `thread` and `process`

#### Scenario: Environment
- **WHEN** `vkxp` is started with `VKXP_GUEST=process` and without `--guest`
- **THEN** the guest runs in a child process
