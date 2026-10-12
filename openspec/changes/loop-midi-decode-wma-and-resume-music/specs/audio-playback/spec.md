## ADDED Requirements

### Requirement: MIDI loop marker
A MIDI track that loops SHALL continue, when it ends, at the last controller 111 it has
played, and at its beginning when it has none. A MIDI track that does not loop SHALL end at its end.

#### Scenario: Marker
- **WHEN** a generated MIDI file with a note before a controller 111 and a different note after it plays as music for twice its length
- **THEN** the first note sounds once and the second note twice

#### Scenario: No marker
- **WHEN** the same file without the controller plays as music for twice its length
- **THEN** both notes sound twice

### Requirement: WMA through an external decoder
Data that starts with the ASF header SHALL be decoded by the `ffmpeg` program named in the
mixer's options and played like other sampled data. When the program cannot be started or
gives nothing, the data SHALL be reported as unsupported with a message that names
`ffmpeg`, and the failure SHALL NOT be fatal.

#### Scenario: Program present
- **WHEN** the options name a stand-in program that writes a known signal, and a play command carries data that starts with the ASF header
- **THEN** the known signal is mixed

#### Scenario: Program absent
- **WHEN** the options name a program that does not exist and the same command arrives
- **THEN** nothing is mixed, the report names `ffmpeg`, and later commands play

### Requirement: Remembered positions
The audio sink SHALL be able to remember under a slot number the file and the position of
the music and of the background sound, and to recall a slot: after a recall, the next play
command on each of the two channels SHALL start at the remembered position when it names
the remembered file, and at the beginning otherwise.

#### Scenario: Same file
- **WHEN** music has played 1000 frames of a file whose frames are a known ramp, slot 0 is remembered, the music is stopped, slot 0 is recalled and the same file is played
- **THEN** the first frame mixed is frame 1000 of the ramp

#### Scenario: Another file
- **WHEN** after the recall another file is played as music
- **THEN** it starts at its beginning, and the first file played after it starts at its beginning too

#### Scenario: Silent channel
- **WHEN** a slot is remembered while no background sound plays, recalled, and a background sound is played
- **THEN** it starts at its beginning
