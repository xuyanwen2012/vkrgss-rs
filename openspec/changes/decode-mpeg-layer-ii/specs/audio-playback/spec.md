## MODIFIED Requirements

### Requirement: Formats
The audio side SHALL decode Ogg Vorbis, WAV (PCM), MP3 and MPEG audio layer II data, and SHALL
render Standard MIDI files with a SoundFont when one is available. The format SHALL be
recognised from the data, not from a file name.

#### Scenario: WAV
- **WHEN** a play command carries a generated one-second 440 Hz mono 16-bit WAV file
- **THEN** the mixer's output for that second is a 440 Hz tone on both output channels

#### Scenario: Ogg and MP3
- **WHEN** a play command carries a small Ogg Vorbis or MP3 file committed as a test fixture
- **THEN** the mixer's output is not silent and has the fixture's duration within a tenth of a second

#### Scenario: Layer II
- **WHEN** a play command carries a small MPEG audio layer II file committed as a test fixture
- **THEN** the mixer's output is not silent and has the fixture's duration within a tenth of a second

#### Scenario: MIDI
- **WHEN** a play command carries a generated MIDI file with one note and a SoundFont is configured
- **THEN** the mixer's output is not silent while the note sounds
