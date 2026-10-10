# Spec Delta

## ADDED Requirements

### Requirement: Stream framing
The protocol SHALL define a framing that carries `GuestToHost` and `HostToGuest` messages over
a byte stream: a length followed by the versioned encoding of one message. Reading SHALL fail
with an error, and not with a panic or an unbounded allocation, on a length above the bound of
the framing, on bytes that do not decode and on a stream that ends inside a message; a stream
that ends between two messages SHALL be reported as the end of the stream.

#### Scenario: Round trip
- **WHEN** a frame packet with a bitmap of 640 by 480 pixels, an input snapshot and a tick are written to a buffer and read back
- **THEN** the three messages read equal the three written, in order

#### Scenario: Truncated message
- **WHEN** the buffer of a written message is cut short by one byte and read
- **THEN** the read reports an error

#### Scenario: Oversized length
- **WHEN** a length above the bound is read
- **THEN** the read reports an error without reading the bytes that would follow

#### Scenario: Other version
- **WHEN** a message encoded with another protocol version is read from the stream
- **THEN** the read reports the version error of the encoding

### Requirement: Guest end over a stream socket
The guest end of the link SHALL have a transport over a connected Unix stream socket with the
operations of the in-process one: send, a receive that waits, and a receive that does not.
A connection closed by the other side SHALL be reported as the disconnection of the link.
The transport SHALL use no thread of its own.

#### Scenario: Messages in order
- **WHEN** the other side of the socket writes an input snapshot and a tick with the framing
- **THEN** two waiting receives of the guest end return the snapshot and then the tick

#### Scenario: Nothing waiting
- **WHEN** the other side has written nothing and the guest end is asked without waiting
- **THEN** it answers that no message is waiting, and a message written afterwards is returned by the next call

#### Scenario: Other side gone
- **WHEN** the other side of the socket is closed
- **THEN** a receive and a send of the guest end report the disconnection
