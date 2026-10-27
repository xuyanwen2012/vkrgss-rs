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
where `v` is the volume, lowered in proportion during a fade. The pitch, a percentage from 1
up, SHALL change the playback speed by that percentage. A stop command SHALL silence its
channel at once. A fade command SHALL lower `v` evenly to 0 over the given duration and then
stop the channel.

#### Scenario: Half speed
- **WHEN** a 440 Hz tone is played with pitch 50
- **THEN** the output tone is 220 Hz and lasts twice as long

#### Scenario: Fade
- **WHEN** music is faded over 1000 ms
- **THEN** the output amplitude after 500 ms is about 0.133 of the amplitude before the fade, and after 1000 ms the channel is silent

#### Scenario: Volume curve
- **WHEN** the same tone is played at volumes 100, 80, 50 and 0
- **THEN** the output amplitudes are 1, about 0.447 and about 0.133 of the track's own, and silence

#### Scenario: Pitch outside 50 to 150
- **WHEN** a 440 Hz tone of one second is played with pitch 200, and again with pitch 25
- **THEN** the output tones are 880 Hz for half a second and 110 Hz for four seconds

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

### Requirement: Start position of a play command
A play command SHALL say at which sample frame of its file the sound starts, and sampled
data SHALL start there, at any pitch. A MIDI file and data decoded by the external decoder
SHALL start at their beginning. A start at or behind the end of a sound that loops SHALL
begin it at its beginning. A recalled position SHALL win over the start of the command.

#### Scenario: Start
- **WHEN** a file at the mixer's rate whose frames are a known ramp is played as music with the start 1000
- **THEN** the first frame mixed is frame 1000 of the ramp

#### Scenario: Start at half the pitch
- **WHEN** the same file is played as background sound with the pitch 50 and the start 1000
- **THEN** the first frame mixed is frame 1000 of the ramp

#### Scenario: Start of zero
- **WHEN** the same file is played with the start 0
- **THEN** the first frame mixed is frame 0 of the ramp

#### Scenario: Behind the end
- **WHEN** a file of 2000 frames is played as music with the start 5000
- **THEN** the first frame mixed is frame 0 of the ramp

#### Scenario: A recalled position wins
- **WHEN** music has played 1000 frames of the ramp, slot 0 is remembered, the music is stopped, slot 0 is recalled and the same file is played with the start 5
- **THEN** the first frame mixed is frame 1000 of the ramp

#### Scenario: MIDI
- **WHEN** a generated MIDI file with one note at its beginning is played with the start 100000 and a SoundFont is configured
- **THEN** the note sounds from the first frame mixed

### Requirement: Volume command
A volume command SHALL set the volume of what its channel plays, without starting it again
and without moving it. On a channel that plays nothing it SHALL do nothing.

#### Scenario: Lower and go on
- **WHEN** the ramp file is played as music at volume 100, 1000 frames are rendered, a volume command of 50 for the music channel is applied and one more frame is rendered
- **THEN** that frame is frame 1000 of the ramp at about 0.133 of its value

#### Scenario: Nothing plays
- **WHEN** a volume command for the background-sound channel is applied while it plays nothing, and a file is then played on it at volume 100
- **THEN** the file sounds at volume 100

### Requirement: Return of the music after a music effect
When a music effect that held the music ends, by itself or by a stop command, and its play
command carried a return time above 0, the volume of the music SHALL rise evenly from 0 to
its value over that time, through the gain of the volume. With a return time of 0 the music
SHALL go on at its volume at once. A stop of the effect channel while no effect plays SHALL
change nothing of the music.

#### Scenario: One second, measured on the original
- **WHEN** a tone plays as music at volume 100, an effect of half a second is played with the return time 1000 ms, and the output is rendered
- **THEN** the music is silent during the effect, its amplitude 500 ms after the end of the effect is about 0.133 of its own, and from 1000 ms after it is its own

#### Scenario: A stopped effect
- **WHEN** the same effect is stopped by a stop command after a tenth of a second
- **THEN** the music rises in the same way from the moment of the stop

#### Scenario: A music at a lower volume
- **WHEN** the music plays at volume 50 and the same effect ends
- **THEN** 500 ms after the end the music has the amplitude of volume 25, and from 1000 ms after it that of volume 50

#### Scenario: No return time
- **WHEN** the effect is played with the return time 0
- **THEN** the first frame after the end of the effect has the music at its own amplitude

#### Scenario: A stop without an effect
- **WHEN** a tone plays as music and a stop command for the effect channel is applied while no effect plays
- **THEN** the amplitude of the music does not change

### Requirement: Fade of no time
A fade command of 0 ms SHALL stop its channel at once. A fade that follows another fade, of any
duration and in the same buffer of the device or not, SHALL lower the volume from the level the
voice has, and the output SHALL never carry a sample that is not a number.

#### Scenario: Fade of zero, then another fade
- **WHEN** music plays, a fade of 0 ms and a fade of 500 ms are applied before the next buffer is rendered, and 600 ms are rendered
- **THEN** every output sample is a number and the music is silent

#### Scenario: Two fades
- **WHEN** music is faded over 1000 ms and, 500 ms later, faded again over 1000 ms
- **THEN** the amplitude does not rise at the second fade and every output sample is a number

### Requirement: MIDI at every pitch
The pitch of a play command SHALL apply to MIDI music at every value of its range as it does
to sampled data, also where the synthesiser cannot run at the rate the pitch asks for; such
a command SHALL NOT be reported as data in an unsupported format.

#### Scenario: High and low pitch
- **WHEN** a generated MIDI file with one note is played with pitch 400, and again with pitch 10, with a SoundFont configured and an output of 48 000 Hz
- **THEN** both are not silent, and no report is made

### Requirement: The device callback does not wait
The function the audio device calls for frames SHALL NOT take a lock that another thread can
hold, SHALL NOT write to the error stream and SHALL NOT free a decoded source. Damage a source
meets while it plays SHALL still be reported once.

#### Scenario: Error stream held
- **WHEN** the error stream blocks while a sound with damaged data is mixed through the output
- **THEN** the callback returns for every buffer, and the damage is reported once the stream is free

### Requirement: A state taken right after a play command
A slot remembered after a play command on music or background sound SHALL hold that file and
its start position, also when the device has not mixed a frame of it yet.

#### Scenario: Before the first buffer
- **WHEN** a file whose frames are a known ramp is played as music from file frame 1000 through the output, slot 0 is remembered before the callback runs, the music is stopped, slot 0 is recalled and the same file is played from its start
- **THEN** the first frame mixed is frame 1000 of the ramp

### Requirement: A failure in the audio thread is not fatal
A panic while a command is prepared on the audio thread SHALL be reported once and SHALL leave
that command's channel silent; later commands SHALL play.

#### Scenario: Panicking source
- **WHEN** preparing a play command panics, and a valid play command follows
- **THEN** one report is made and the second sound is mixed
