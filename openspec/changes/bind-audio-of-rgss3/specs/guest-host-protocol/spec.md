## ADDED Requirements

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
