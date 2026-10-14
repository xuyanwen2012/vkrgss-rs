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

### Requirement: Window blend mode
The blend modes of a quad SHALL include a mode for window layers besides normal, additive and
subtractive. The protocol version SHALL be 4.

#### Scenario: Round trip
- **WHEN** a frame packet with a quad in the window blend mode is serialised and read back
- **THEN** the quad is equal to the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 4

### Requirement: Raw key and pointer state
An input snapshot SHALL carry, besides the buttons, one bit for each of the 256 Windows
virtual-key codes and an optional pointer position in logical frame pixels, all plain data.
The empty snapshot SHALL have no key held and no pointer. The protocol version SHALL be 5.

#### Scenario: Round trip
- **WHEN** a snapshot with the virtual keys 0x01 and 0x41 held and the pointer at (100, 50) is serialised and read back
- **THEN** it equals the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 5

### Requirement: Table resources
A frame packet SHALL be able to create, replace and destroy tables: arrays of 16-bit signed
integers with one to three dimensions, identified by numbers of their own, applied in order
before any op is drawn, like bitmap changes. An op that names a table the host does not have
SHALL make the packet fail with a typed error and produce no frame.

#### Scenario: Round trip
- **WHEN** a packet that creates a 20 by 15 by 3 table and a 400-entry table is serialised and read back
- **THEN** it equals the original

#### Scenario: Unknown table
- **WHEN** a tilemap op names a table that was never created
- **THEN** the backend returns a typed error and produces no frame

### Requirement: Tilemap layer op
The tilemap layer op SHALL be a plain-data op with a clip rectangle, the tileset bitmap, up to
seven autotile bitmaps, the table of tile numbers, the table of priorities, the scroll origin,
the map layer from 0 to 2, the priority from 0 to 5 that it draws, and the autotile animation
step. The protocol version SHALL be 6, and the op SHALL no longer be a reserved kind.

#### Scenario: Round trip
- **WHEN** a packet with a tilemap layer op is serialised and read back
- **THEN** the op equals the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 6

### Requirement: Every op kind is implemented
The protocol SHALL contain only op kinds that both backends evaluate. The protocol version
SHALL be 7.

#### Scenario: Version 6 stream
- **WHEN** a decoder reads a stream that declares protocol version 6
- **THEN** it returns the unsupported-version error naming 6 and 7

#### Scenario: Round trip of each kind
- **WHEN** one op of each kind is encoded and decoded
- **THEN** each comes back equal, and both backends accept a packet that holds them

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

### Requirement: State messages
The protocol SHALL carry, from the host, a request to save a state into a slot, a request to
load one, and the order to a parked copy to resume; and from the guest, the attachment of a
process with its process id and, for a parked copy, its slot, the outcome of a game, and the
three answers saved, loaded and failed with their slot, the failed one saying whether a save
or a load failed. The protocol version SHALL be 10.

#### Scenario: Round trip
- **WHEN** one message of each of these kinds is encoded and decoded, the failed answer once for a save and once for a load
- **THEN** each decoded message equals the original

#### Scenario: Older stream
- **WHEN** a stream encoded with version 9 is decoded
- **THEN** it is rejected with the version error

### Requirement: Injection and pointer messages
The protocol SHALL carry, from the guest, the press or release of a Windows virtual-key
code on the player's behalf, and whether the pointer is to be seen over the picture.

#### Scenario: Round trip
- **WHEN** a press of key 0x0D, a release of key 0x01 and both values of the pointer message are encoded and decoded
- **THEN** each decoded message equals the original

### Requirement: Brightness of a frame packet
A frame packet SHALL carry a brightness, a whole number from 0 to 255, where 255 means that
the frame is shown as composed.

#### Scenario: Brightness travels with every packet
- **WHEN** the guest builds two packets with the brightness 255 and 100 and both are encoded and decoded
- **THEN** each decoded packet reports the brightness it was built with

### Requirement: Rule of a transition message
A transition message SHALL say by which rule its frames are stepped: that of RGSS1 or that of
RGSS2.

#### Scenario: Rule travels with the message
- **WHEN** two transition messages with the same duration and the two rules are encoded and decoded
- **THEN** each decoded message reports the rule it was built with

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version

### Requirement: Tilemap plane op of RGSS2
The protocol SHALL have an op for one plane of a tilemap of RGSS2 that carries which plane it
is, its clip and corner, the first cell, the animation step, the flash level, the ids of the
map, passages and flash tables, and up to nine bitmap ids. All its values SHALL be integers.

#### Scenario: Round trip
- **WHEN** a packet with a lower and an upper plane op, some bitmap slots empty, is encoded and decoded
- **THEN** the decoded packet equals the one built

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version

### Requirement: Rule set of the tilemap plane op
The tilemap plane op SHALL carry a rule set, that of RGSS2 or that of RGSS3, as an integer
value like its other fields. With the rule set of RGSS3 the table id it carries for the
passages SHALL name the flags table, and the map table SHALL be sent with all its layers.
The protocol version SHALL be one above the version before this change.

#### Scenario: Round trip
- **WHEN** a packet with a lower plane op of the rule set of RGSS2 and an upper plane op of the rule set of RGSS3, some bitmap slots empty, is encoded and decoded
- **THEN** the decoded packet equals the one built, rule sets included

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version, which states both versions

#### Scenario: Four layers
- **WHEN** a table of 4 by 3 by 4 elements is sent as the map of a plane op of the rule set of RGSS3 and decoded
- **THEN** the decoded table has its 48 elements in the order sent

#### Scenario: Documents name the version
- **WHEN** `README.md` and `docs/architecture.md` are read where they state the protocol version
- **THEN** both give the value of `PROTOCOL_VERSION`, and `docs/architecture.md` says that this version added the rule set of the tilemap plane op

### Requirement: Rule of a freeze message
A freeze message SHALL say by which rule the frozen picture is kept: as the frame at its own
size, which is what every freeze meant before this change, or in the buffer of RGSS3. The
rule SHALL be an integer value like the rule of a transition message. The protocol version
SHALL be one above the version before this change.

#### Scenario: Rule travels with the message
- **WHEN** two freeze messages with the two rules are encoded and decoded
- **THEN** each decoded message reports the rule it was built with

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version, which states both versions

#### Scenario: Documents name the version
- **WHEN** `README.md` and `docs/architecture.md` are read where they state the protocol version
- **THEN** both give the value of `PROTOCOL_VERSION`, and `docs/architecture.md` says that this version added the rule of the freeze message

### Requirement: Start, return time and volume command of audio
A play command SHALL carry the sample frame at which its sound starts and the time in
milliseconds over which the music returns after a music effect, both whole numbers, and its
pitch SHALL be a whole number of percent from 1 to 65535. The protocol SHALL have a volume
command that names a channel and a volume from 0 to 100. The protocol version SHALL be one
above the version before this change.

#### Scenario: Round trip
- **WHEN** a play command with the start 123456789012, the return time 1000 and the pitch 453, and a volume command for the music channel with the volume 80, are encoded and decoded
- **THEN** each decoded command equals the one built

#### Scenario: Every audio command survives encoding
- **WHEN** one play, one stop, one fade and one volume command are sent as messages of the guest, encoded and decoded
- **THEN** each decoded message is of the same kind and equal to the original

#### Scenario: Version
- **WHEN** a stream of the protocol version before this change is read
- **THEN** it is refused with the error for an unsupported version, which states both versions
