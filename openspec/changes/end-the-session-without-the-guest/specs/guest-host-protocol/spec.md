## MODIFIED Requirements

### Requirement: Guest end over a stream socket
The guest end of the link SHALL have a transport over a connected Unix stream socket with the
operations of the in-process one: send, a receive that waits, and a receive that does not.
A connection closed by the other side SHALL be reported as the disconnection of the link.
The transport SHALL use no thread of its own, and the receive that does not wait SHALL not
change the mode of the socket.

#### Scenario: Messages in order
- **WHEN** the other side of the socket writes an input snapshot and a tick with the framing
- **THEN** two waiting receives of the guest end return the snapshot and then the tick

#### Scenario: Nothing waiting
- **WHEN** the other side has written nothing and the guest end is asked without waiting
- **THEN** it answers that no message is waiting, and a message written afterwards is returned by the next call

#### Scenario: Other side gone
- **WHEN** the other side of the socket is closed
- **THEN** a receive and a send of the guest end report the disconnection

#### Scenario: Send while polling
- **WHEN** one thread sends frame packets larger than the socket's buffer through the split sender while another thread asks without waiting again and again, and the other side reads slowly
- **THEN** every packet arrives whole and in order, and no send reports the disconnection

## ADDED Requirements

### Requirement: Incremental reading of the framing
The framing SHALL have a reader that takes bytes as they arrive and gives each message once
its last byte is there, with the errors and the bound of reading from a stream. It SHALL
keep no more bytes than the bound of one message plus what was given to it at once.

#### Scenario: One byte at a time
- **WHEN** two written messages are given to the reader one byte at a time
- **THEN** it gives the first message after its last byte, and then the second, and nothing before

#### Scenario: Oversized length
- **WHEN** the reader is given a length above the bound
- **THEN** it reports the error without waiting for the bytes that would follow
