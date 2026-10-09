# guest-host-protocol Specification

## Purpose
Defines the plain-data messages exchanged between the guest (script) side and the host (window, GPU, audio) side, and their versioned binary encoding.

## Requirements

### Requirement: Plain-data boundary
Every message exchanged between the guest and host sides SHALL be self-contained plain data that
can be encoded to bytes and decoded in another process without loss.

#### Scenario: Round trip preserves a message
- **WHEN** any guest-to-host or host-to-guest message is encoded and then decoded
- **THEN** the decoded message equals the original

#### Scenario: Encoding is deterministic
- **WHEN** the same message is encoded twice
- **THEN** the two byte sequences are identical

### Requirement: Versioned encoding
Encoded message streams SHALL begin with a magic value and a protocol version, and a decoder
SHALL reject streams whose magic or version it does not support.

#### Scenario: Unknown version is rejected
- **WHEN** a decoder reads a stream with an unsupported protocol version
- **THEN** it returns an error that states the found and supported versions and decodes nothing

### Requirement: Frame packet content
A frame packet SHALL carry a frame index, the logical screen size, the frame rate the guest runs
at, the bitmap changes since the previous packet and an ordered list of draw ops. The order of
the op list SHALL be the draw order. The frame rate SHALL be a whole number of frames per second
from 10 to 120, and a value outside that range SHALL NOT be representable in a decoded packet.

#### Scenario: Bitmap is defined before use
- **WHEN** a packet's op refers to a bitmap created by a delta in the same or an earlier packet
- **THEN** a backend can resolve the reference

#### Scenario: Stale bitmap reference is detectable
- **WHEN** an op refers to a bitmap id whose bitmap was destroyed, even if the slot was reused
- **THEN** a backend reports an invalid-reference error instead of drawing the new occupant

#### Scenario: Frame rate travels with every packet
- **WHEN** the guest builds two packets with different frame rates and both are encoded and decoded
- **THEN** each decoded packet reports the frame rate it was built with

#### Scenario: Frame rate out of range is rejected
- **WHEN** an encoded stream holds a packet whose frame rate is below 10 or above 120
- **THEN** decoding fails and yields no packet

### Requirement: No floating point across the boundary
Geometry, colour and opacity values in messages SHALL be integers or fixed-point integers.

#### Scenario: Packet types expose no float fields
- **WHEN** the protocol types are inspected
- **THEN** no field reachable from a message has a floating-point type

### Requirement: Reserved ops are rejected explicitly
A backend that receives an op kind it does not implement SHALL return a typed unsupported-op
error naming the kind, and SHALL NOT silently skip it.

#### Scenario: Reserved op kind
- **WHEN** a packet contains a reserved op kind
- **THEN** the backend returns an unsupported-op error and produces no frame

### Requirement: Input snapshot
The host SHALL deliver input to the guest as a snapshot value containing the complete logical
button state, and a recorded sequence of snapshots SHALL be replayable as input. The logical
buttons SHALL be the twenty `Input` constants of RGSS1: the four directions, A, B, C, X, Y, Z,
L, R, SHIFT, CTRL, ALT and F5 to F9, each identified by the numeric value of its constant.

#### Scenario: Recording replays identically
- **WHEN** a sequence of input snapshots is encoded to a file and decoded again
- **THEN** the decoded sequence equals the recorded one in content and order

#### Scenario: Every RGSS1 button is representable
- **WHEN** each of the twenty buttons in turn is the only one held in a snapshot
- **THEN** the snapshot reports that button as held and the other nineteen as not held

#### Scenario: F9 has the value of its constant
- **WHEN** the button F9 is inspected
- **THEN** its numeric value is 29

### Requirement: Control messages
Besides frame packets, the guest SHALL be able to tell the host to freeze the display, start a
transition, issue an audio command, show a modal message, set the window title, restart frame
pacing and shut down. The host SHALL be able to send the guest an input snapshot, the dismissal
of a modal message, a tick, a reset request and a quit request. A reset request and a quit
request SHALL be distinct messages.

#### Scenario: Every control message survives encoding
- **WHEN** one message of each guest-to-host and host-to-guest kind is encoded and decoded
- **THEN** each decoded message is of the same kind and equal to the original

#### Scenario: Reset is not quit
- **WHEN** a reset request and a quit request are compared after a round trip
- **THEN** they are different messages

### Requirement: Link between the sides
The protocol SHALL provide a link with a guest endpoint and a host endpoint. Messages sent on
one endpoint SHALL arrive at the other in the order they were sent, the guest endpoint SHALL be
able to wait for the next host message, and the host SHALL be notified whenever the guest has
sent a message. When one endpoint is dropped, sending or waiting on the other SHALL report a
disconnection instead of blocking forever.

#### Scenario: Order is kept
- **WHEN** the guest sends three messages and the host then reads the link
- **THEN** the host receives the three messages in the order sent

#### Scenario: Host is woken
- **WHEN** the guest sends a message
- **THEN** the host's wake hook has been called at least once by the time the send returns

#### Scenario: Peer has gone
- **WHEN** the host endpoint is dropped while the guest waits for a message
- **THEN** the wait ends with a disconnection error

### Requirement: Tick handshake
The host SHALL answer every frame message with exactly one tick, and SHALL NOT send a tick that
answers no frame. A guest SHALL NOT send a second frame message before it has received the tick
for the first.

#### Scenario: One tick per frame
- **WHEN** a guest sends N frame messages, each after receiving the previous tick
- **THEN** it receives exactly N ticks

#### Scenario: No unsolicited tick
- **WHEN** the guest sends no frame message for a long time
- **THEN** the host sends no tick

### Requirement: Quad sampling variants
A quad SHALL state how it is sampled as one of two plain-data variants: an affine transform, as
before, or a stretch, which names an integer destination rectangle in logical pixels and a
mirror flag. The protocol version SHALL be 3.

#### Scenario: Round trip
- **WHEN** a frame packet with one affine quad and one stretch quad is serialised and read back
- **THEN** both quads are equal to the originals

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 3
