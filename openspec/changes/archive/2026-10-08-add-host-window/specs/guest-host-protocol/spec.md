## ADDED Requirements

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
