# Spec Delta

## ADDED Requirements

### Requirement: State messages
The protocol SHALL carry, from the host, a request to save a state into a slot, a request to
load one, and the order to a parked copy to resume; and from the guest, the attachment of a
process with its process id and, for a parked copy, its slot, the outcome of a game, and the
three answers saved, loaded and failed with their slot. The protocol version SHALL be 8.

#### Scenario: Round trip
- **WHEN** one message of each of these kinds is encoded and decoded
- **THEN** each decoded message equals the original

#### Scenario: Older stream
- **WHEN** a stream encoded with version 7 is decoded
- **THEN** it is rejected with the version error
