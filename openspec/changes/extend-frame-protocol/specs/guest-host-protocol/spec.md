## MODIFIED Requirements

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

## ADDED Requirements

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
