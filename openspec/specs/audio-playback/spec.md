# audio-playback Specification

## Purpose
Defines how the host turns audio commands into sound: the formats decoded, the behaviour of the
four channels, and what happens when something cannot be played.

## Requirements

### Requirement: Formats
The audio side SHALL decode Ogg Vorbis, WAV (PCM) and MP3 data, and SHALL render Standard MIDI
files with a SoundFont when one is available. The format SHALL be recognised from the data,
not from a file name.

#### Scenario: WAV
- **WHEN** a play command carries a generated one-second 440 Hz mono 16-bit WAV file
- **THEN** the mixer's output for that second is a 440 Hz tone on both output channels

#### Scenario: Ogg and MP3
- **WHEN** a play command carries a small Ogg Vorbis or MP3 file committed as a test fixture
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
