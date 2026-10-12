## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Injection and pointer messages
The protocol SHALL carry, from the guest, the press or release of a Windows virtual-key
code on the player's behalf, and whether the pointer is to be seen over the picture.

#### Scenario: Round trip
- **WHEN** a press of key 0x0D, a release of key 0x01 and both values of the pointer message are encoded and decoded
- **THEN** each decoded message equals the original
