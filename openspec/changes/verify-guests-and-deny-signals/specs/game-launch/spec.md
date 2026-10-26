## ADDED Requirements

### Requirement: Identity of a process that attaches
The program SHALL take the process number and user of every connection to the guest socket
from the kernel, SHALL refuse a connection whose user is not its own, whose number is not
positive or differs from the number its attachment states, SHALL require the first
connection to come from the guest process it started, and SHALL send signals only to
numbers taken this way.

#### Scenario: Stated number differs
- **WHEN** a process connects to the guest socket and attaches stating the number of another process, or the number 0
- **THEN** the connection is refused with a warning, and no signal is sent to the stated number

#### Scenario: First connection from another process
- **WHEN** the first process that attaches is not the child the program started
- **THEN** the start of the guest fails with an error that says so, and the child is ended

#### Scenario: Copies still attach
- **WHEN** a headless run with `--guest process` and `--states "40:save 0, 80:load 0"` runs
- **THEN** it dumps the same frames as before the change and no process of the run is left

### Requirement: Signals of a confined guest process
A confined guest process on Linux and Android SHALL be refused, with `EPERM`, the system calls
`kill`, `tkill`, `rt_sigqueueinfo`, `rt_tgsigqueueinfo` and `pidfd_send_signal`, and where
the kernel has Landlock's signal scope it SHALL signal no process outside its Landlock
domain. Signals a guest sends its own threads SHALL still arrive.

#### Scenario: Signal to another process
- **WHEN** code in a process that has installed the guest's filter calls `kill` with the number of another process of the same user, or with 0
- **THEN** the call fails with `EPERM` and the other process is still running

#### Scenario: Own threads
- **WHEN** that process calls `raise(SIGUSR1)` with a handler installed, and a thread of it sends `SIGUSR1` to another of its threads with `pthread_kill`
- **THEN** both signals are handled

#### Scenario: Script ends by its signal
- **WHEN** a script run with the confinement does not rescue `Interrupt` and gets SIGINT, over Ruby 1.8.7 and over Ruby 1.9.2
- **THEN** the process ends by SIGINT, as before the change

### Requirement: State answers are made by the program
Of the messages a running guest process sends, the program SHALL hand on no attachment
and no answer about states except a failed save; it SHALL make the answers saved, loaded
and failed load itself. A guest that cannot be copied still answers its own failed save.

#### Scenario: Guest sends a loaded answer
- **WHEN** the running guest process sends a loaded answer for slot 0 without a load request
- **THEN** the host receives no loaded answer, and the next frame of the guest receives its tick

#### Scenario: Guest that cannot be copied
- **WHEN** a guest process started with process functions allowed is asked to save a state
- **THEN** the host receives its failed save answer and says that no state could be saved

### Requirement: Load while a slot is refilled
A load of the slot whose copy was just resumed SHALL wait until the resumed guest has left
its new copy, and then load it; it SHALL fail only if no new copy attaches.

#### Scenario: Load again at once
- **WHEN** the relay is asked to load slot 0, and asked again before the resumed guest's new copy has attached
- **THEN** after that copy attaches the state of slot 0 is loaded again and the host receives a loaded answer, not a failed one
