# audio-playback Specification

## Purpose
Defines how the host turns audio commands into sound: the formats decoded, the behaviour of the
four channels, and what happens when something cannot be played.

## Requirements

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

### Requirement: Channels
Music and background sound SHALL each play one track that loops until stopped or replaced. A
music effect SHALL play once, SHALL pause the music while it plays and SHALL let the music
continue from where it paused when it ends. Sound effects SHALL play once each and several
SHALL be able to sound together. A play command on music, background sound or music effect
SHALL replace what that channel was playing.

#### Scenario: Loop
- **WHEN** a half-second track is played as music and two seconds are rendered
- **THEN** the output is not silent in the last half second

#### Scenario: Sound effects overlap
- **WHEN** two sound effects are started a tenth of a second apart
- **THEN** both are audible in the output during their overlap

#### Scenario: Music effect pauses music
- **WHEN** a music effect is started while music plays
- **THEN** the music contributes nothing to the output until the effect has ended, and continues afterwards

### Requirement: Volume, pitch, stop and fade
The volume of a play command (0 to 100) SHALL set the track's gain to
`10 ^ (-0.0175 * (100 - v))`, 0.35 dB below full for every unit under 100, and to silence for 0,
where `v` is the volume, lowered in proportion during a fade. The pitch (50 to 150) SHALL
change the playback speed by that percentage. A stop command SHALL silence its channel at once.
A fade command SHALL lower `v` evenly to 0 over the given duration and then stop the channel.

#### Scenario: Half speed
- **WHEN** a 440 Hz tone is played with pitch 50
- **THEN** the output tone is 220 Hz and lasts twice as long

#### Scenario: Fade
- **WHEN** music is faded over 1000 ms
- **THEN** the output amplitude after 500 ms is about 0.133 of the amplitude before the fade, and after 1000 ms the channel is silent

#### Scenario: Volume curve
- **WHEN** the same tone is played at volumes 100, 80, 50 and 0
- **THEN** the output amplitudes are 1, about 0.447 and about 0.133 of the track's own, and silence

### Requirement: Failures are not fatal
Data in an unsupported or damaged format, a MIDI file without a SoundFont, and an output device
that cannot be opened SHALL each be reported once and SHALL leave the game running without that
sound.

#### Scenario: Unsupported data
- **WHEN** a play command carries bytes that are no supported format
- **THEN** one report is made, the channel stays silent and later commands still work

#### Scenario: No device
- **WHEN** no audio output device can be opened
- **THEN** one report is made and every command is accepted and ignored

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
