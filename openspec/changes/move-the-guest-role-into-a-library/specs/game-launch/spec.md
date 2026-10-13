## MODIFIED Requirements

### Requirement: Guest in a process
`vkxp` SHALL run the guest either as a thread of its own process or in a child process. In a
child process the guest SHALL exchange with the host, over a socket that other users of the
machine cannot reach, the statement of its version at the start of each connection it
opens, the messages of the protocol, and nothing else. A run SHALL write the same frames in
both forms, and its exit status SHALL follow the same rules.

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

## ADDED Requirements

### Requirement: Version of a guest process
A guest process SHALL state the version of its executable as the first bytes of each
connection it opens to the program, before its first message. The program SHALL refuse a
connection that states another version than its own, or none, with an error that names its
own version and the version stated; when it is the connection of the guest process the
program has just started, the run SHALL end with that error.

#### Scenario: Same version
- **WHEN** a game is run headless with `--guest process` and a states schedule that saves and loads
- **THEN** the guest process and its copies attach and the run exits with status 0

#### Scenario: Other version
- **WHEN** a process connects to the socket of the program, states the version `0.0.0` and sends an attachment
- **THEN** the attachment is refused with an error that contains `0.0.0` and the version of the program

#### Scenario: No statement
- **WHEN** a process connects to the socket of the program and sends an attachment without stating a version
- **THEN** the attachment is refused with an error that names the version of the program

#### Scenario: Statement and message in one write
- **WHEN** a guest writes the statement of the program's version and its attachment to the connection at once
- **THEN** the program reads the statement and then the attachment, whole

### Requirement: Executable of the guest process
The program SHALL take the executable it starts as its guest process from the runtime of
the game. For RGSS1 and for RGSS2 it SHALL be the executable of the program itself.

#### Scenario: XP and VX
- **WHEN** the executable of the guest process is asked for RGSS1 and for RGSS2
- **THEN** both answers are the path of the running executable
