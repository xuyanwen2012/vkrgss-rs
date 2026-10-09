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
A frame packet SHALL carry a frame index, the logical screen size, the bitmap changes since the
previous packet and an ordered list of draw ops. The order of the op list SHALL be the draw
order.

#### Scenario: Bitmap is defined before use
- **WHEN** a packet's op refers to a bitmap created by a delta in the same or an earlier packet
- **THEN** a backend can resolve the reference

#### Scenario: Stale bitmap reference is detectable
- **WHEN** an op refers to a bitmap id whose bitmap was destroyed, even if the slot was reused
- **THEN** a backend reports an invalid-reference error instead of drawing the new occupant

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
button state, and a recorded sequence of snapshots SHALL be replayable as input.

#### Scenario: Recording replays identically
- **WHEN** a sequence of input snapshots is encoded to a file and decoded again
- **THEN** the decoded sequence equals the recorded one in content and order
