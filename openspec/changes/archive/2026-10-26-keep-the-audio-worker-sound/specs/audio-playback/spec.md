## ADDED Requirements

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
