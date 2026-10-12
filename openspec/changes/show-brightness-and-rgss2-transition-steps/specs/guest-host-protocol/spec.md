## ADDED Requirements

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
