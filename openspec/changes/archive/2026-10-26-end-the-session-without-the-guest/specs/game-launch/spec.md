## ADDED Requirements

### Requirement: The relay does not wait on a guest
The program SHALL read and write a guest process without blocking on it. A guest process
that stops inside a message, or stops reading what the program sends, SHALL be ended once
the program's deadline for the guest has passed, and SHALL not keep the program from ending.

#### Scenario: Stopped inside a message
- **WHEN** a stand-in guest process attaches, writes half of a frame packet and then waits for ever, and the host ends
- **THEN** the relay returns within 10 seconds, reports that the guest was given up on, and the stand-in is ended

#### Scenario: Not reading
- **WHEN** a stand-in guest process attaches and never reads, and the host sends it messages until the socket's buffer is full and then ends
- **THEN** the relay returns within 10 seconds and the stand-in is ended

### Requirement: Attaching does not hold the game
A connection to the guest socket SHALL be attached without stopping the messages of the
running guest for longer than the wait for an attachment, and an error of accepting
connections SHALL be reported once without busy waiting.

#### Scenario: Silent connection
- **WHEN** a process connects to the guest socket and sends nothing while the running guest sends frames
- **THEN** the frames reach the host, the connection is dropped after the wait for an attachment, and a warning says so

### Requirement: Bound on what a guest process queues
Between two ticks the program SHALL hand the host at most 256 MiB of messages from a guest
process that are not frame packets. A guest process that sends more SHALL be ended with a
report that names the bound.

#### Scenario: Flood of titles
- **WHEN** a stand-in guest process sends title messages without end and no frame
- **THEN** it is ended after 256 MiB, the report names the bound, and the program's memory stays below that bound plus 64 MiB
