## MODIFIED Requirements

### Requirement: Audio playback
In RGSS1 and RGSS2 `Audio.bgm_play`, `bgs_play`, `me_play` and `se_play` SHALL take a
filename, a volume (default 100, clamped to 0 to 100) and a pitch (default 100, clamped to
50 to 150). In every runtime they SHALL find the file as audio through the game's files and
send the host a play command for the matching channel with the file's contents, and a file
that is not found SHALL raise `Errno::ENOENT` with the filename as given.

#### Scenario: Play
- **WHEN** `Audio.se_play("Audio/SE/001-System01", 80)` is evaluated and `Audio/SE/001-System01.ogg` exists in the run-time package
- **THEN** the host receives a play command for the sound-effect channel with volume 80, pitch 100 and the bytes of that file

#### Scenario: Missing file
- **WHEN** `Audio.bgm_play("Audio/BGM/Nope")` is evaluated
- **THEN** `Errno::ENOENT` is raised with the message `No such file or directory - Audio/BGM/Nope`

#### Scenario: Same music again
- **WHEN** `Audio.bgm_play` is called twice in a row with the same filename, volume and pitch
- **THEN** only one play command is sent

#### Scenario: Fields that RGSS3 uses
- **WHEN** a game of RGSS1 or of RGSS2 evaluates `Audio.bgm_play(name, 80)` and `Audio.me_play(name)` for a file that exists
- **THEN** both play commands carry the start 0 and the return time 0, and no volume command is sent

## ADDED Requirements

### Requirement: Functions of Audio in RGSS3
In a game of RGSS3 `Audio.bgm_play` and `Audio.bgs_play` SHALL take a filename and up to
three of volume, pitch and `pos`; `me_play` and `se_play` a filename and up to two of volume
and pitch; `Audio.bgm_pos`, `Audio.bgs_pos` and `Audio.setup_midi` SHALL take no argument.
All SHALL return `nil` except the two positions, which return an Integer. In RGSS1 and RGSS2
the three new functions SHALL NOT be defined and a fourth argument SHALL be refused.

#### Scenario: Surface, measured on the original
- **WHEN** a game of RGSS3 evaluates `(Audio.singleton_methods - [:__reset__]).sort.map { |m| [m, Audio.method(m).arity] }`
- **THEN** the result is `[[:bgm_fade, 1], [:bgm_play, -1], [:bgm_pos, 0], [:bgm_stop, 0], [:bgs_fade, 1], [:bgs_play, -1], [:bgs_pos, 0], [:bgs_stop, 0], [:me_fade, 1], [:me_play, -1], [:me_stop, 0], [:se_play, -1], [:se_stop, 0], [:setup_midi, 0]]`

#### Scenario: Four arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 80, 100, 0)` and `Audio.bgs_play(name, 80, 100, 0)` for a file that exists
- **THEN** both return `nil` and the host receives a play command for the music channel and one for the background-sound channel, each with volume 80, pitch 100 and the bytes of the file

#### Scenario: Before anything plays, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Audio.bgm_pos, Audio.bgs_pos, Audio.setup_midi, Audio.setup_midi]` before any sound was played
- **THEN** the result is `[0, 0, nil, nil]` and the host receives no audio command

#### Scenario: Counts, measured on the original
- **WHEN** a game of RGSS3 calls `Audio.bgm_play` and `Audio.bgs_play` with no argument and with five, and `Audio.me_play` and `Audio.se_play` with four
- **THEN** each raises `ArgumentError`, with the messages `wrong number of arguments (1 for 0)`, `wrong number of arguments (4 for 5)` and `wrong number of arguments (3 for 4)`

#### Scenario: Kinds of arguments, measured on the original
- **WHEN** a game of RGSS3 calls `Audio.bgm_play` for a file that exists with the volume `nil`, with the volume `"a"`, with the pitch `true`, with the `pos` `"a"` and with the `pos` `2**31`
- **THEN** the first raises `TypeError` with the message `no implicit conversion from nil to integer`, the second `TypeError` with `can't convert String into Integer`, the third `TypeError` with `can't convert true into Integer`, the fourth `TypeError` and the fifth `RangeError`

#### Scenario: Forms that are accepted, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 99.9, 100.7, nil)` and then `Audio.bgs_play(name, 100, 100, 44100.7)` for a file of 44100 Hz in stereo
- **THEN** both return `nil`, the first play command carries volume 99, pitch 100 and the start 0, and the second the start 11025

#### Scenario: Missing file while a music plays, measured on the original
- **WHEN** a game of RGSS3 plays a music, shows 60 frames, calls `Audio.bgm_play` with a name that no file has, rescues the error and shows 12 more frames
- **THEN** the error is `Errno::ENOENT` with the name as given, the host received one play command, and `Audio.bgm_pos` is greater than it was before the failed call

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 or of RGSS2 evaluates `[:bgm_pos, :bgs_pos, :setup_midi].map { |m| Audio.respond_to?(m) }` and `Audio.bgm_play(name, 100, 100, 0)` for a file that exists
- **THEN** the first is `[false, false, false]` and the second raises `ArgumentError`

### Requirement: Volume, pitch and start of a play command of RGSS3
In a game of RGSS3 a play command SHALL carry the volume 100 for a volume argument of 100 or
more or below 0, and the argument otherwise; the pitch 100 for an argument below 1 or one
that asks for more than 200000 Hz of a file whose rate is known, and the argument otherwise;
and as its start the sample frame of `pos`, 0 for `nil`, for a value below 0 and for a file
whose format is not known. The command of a music effect SHALL carry a return time of
1000 ms.

#### Scenario: Volume, measured on the original
- **WHEN** a game of RGSS3 plays a sound effect with the volumes -1000, -1, 0, 1, 99, 100, 101 and 2147483647
- **THEN** the play commands carry the volumes 100, 100, 0, 1, 99, 100, 100 and 100

#### Scenario: Pitch, measured on the original
- **WHEN** a game of RGSS3 plays a WAV file of 44100 Hz as a sound effect with the pitches -1, 0, 1, 49, 151, 200, 453, 454 and 1000, and one of 22050 Hz with 907 and 1000
- **THEN** the play commands carry the pitches 100, 100, 1, 49, 151, 200, 453, 100 and 100, and 907 and 100

#### Scenario: Start
- **WHEN** a game of RGSS3 plays a WAV file of 44100 Hz in stereo as music with the `pos` 0, 3, 4, 100001 and -44100, each after `Audio.bgm_stop`, and a WAV file of 22050 Hz in mono with the `pos` 22050
- **THEN** the play commands carry the starts 0, 0, 1, 25000 and 0, and 11025

#### Scenario: A file whose format is not known
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 100, 1000, 100000)` for a file that begins with the header of a Standard MIDI file
- **THEN** the play command carries the pitch 1000 and the start 0

#### Scenario: Return time
- **WHEN** a game of RGSS3 evaluates `Audio.me_play(name); Audio.bgm_play(name); Audio.se_play(name)`
- **THEN** the command of the music effect carries the return time 1000 and the two others the return time 0

### Requirement: The same music again in RGSS3
In a game of RGSS3 a `bgm_play` or `bgs_play` whose filename and pitch equal those of what
its channel plays SHALL send no play command and SHALL leave the position where it is,
whatever `pos` it gives; when its volume differs it SHALL send a volume command. After a
stop or a fade of the channel, and with another filename or pitch, the sound SHALL start
anew at its `pos`.

#### Scenario: Same again, measured on the original
- **WHEN** a game of RGSS3 plays a streamed music of 44100 Hz in stereo, shows 60 frames at 60 frames per second and evaluates `Audio.bgm_play(name, 100, 100, 300000); Audio.bgm_pos`
- **THEN** the host has received one play command and no volume command, and the position is 132304

#### Scenario: Another volume, measured on the original
- **WHEN** after the same 60 frames the game evaluates `Audio.bgm_play(name, 80); Audio.bgm_pos`
- **THEN** the host receives a volume command for the music channel with the volume 80 and no second play command, and the position is 132304

#### Scenario: Another pitch, measured on the original
- **WHEN** after the same 60 frames the game evaluates `Audio.bgm_play(name, 100, 150); Audio.bgm_pos`
- **THEN** the host receives a second play command with the pitch 150 and the position is 0

#### Scenario: Another spelling of the name, measured on the original
- **WHEN** after the same 60 frames the game plays the file under its name in another letter case
- **THEN** the host receives a second play command and the position is 0

#### Scenario: After a fade, measured on the original
- **WHEN** a game of RGSS3 plays a music, shows 60 frames, evaluates `Audio.bgm_fade(1000)`, shows 120 frames and evaluates `Audio.bgm_play` with the same arguments and then `Audio.bgm_pos`
- **THEN** the host receives a second play command and the position is 0

#### Scenario: Background sound
- **WHEN** a game of RGSS3 evaluates `Audio.bgs_play(name, 100); Audio.bgs_play(name, 50); Audio.bgs_play(name, 50)`
- **THEN** the host receives one play command and one volume command with the volume 50 for the background-sound channel

### Requirement: Position of the music and of the background sound
In a game of RGSS3 `Audio.bgm_pos` and `Audio.bgs_pos` SHALL return the position of the
model of what their channel plays, in bytes of 16-bit PCM: 0 before the first play and for a
sound whose format is not known, the last value after a stop, and a value that goes on
during a fade. The guest SHALL compute it from the frames it has shown since the play and
SHALL NOT ask the host, so that it is the same with any audio sink and in every run with
the same input.

#### Scenario: At once, measured on the original
- **WHEN** a game of RGSS3 evaluates `Audio.bgm_play(name, 0, 100, p); Audio.bgm_pos` for a WAV file of 20 s at 44100 Hz in stereo and `p` of 0, 1, 44100, 100000 and 100001, each after `Audio.bgm_stop`
- **THEN** the positions are 0, 0, 11024, 66924 and 66925

#### Scenario: Every sixth frame, measured on the original
- **WHEN** a game of RGSS3 at 60 frames per second plays that file as music and reads `Audio.bgm_pos` at once and after every six calls of `Graphics.update`, fifteen readings in all
- **THEN** the readings are 0, 0, 0, 0, 33076, 33076, 66152, 66152, 99228, 99228, 132304, 132304, 165380, 165380 and 198456

#### Scenario: Background sound and both together, measured on the original
- **WHEN** a game of RGSS3 plays that file as music and as background sound, shows 60 frames and evaluates `[Audio.bgm_pos, Audio.bgs_pos]`
- **THEN** the result is `[132304, 132304]`

#### Scenario: Frames of a wait
- **WHEN** a game of RGSS3 plays that file as music and evaluates `Graphics.wait(60); Audio.bgm_pos`
- **THEN** the position is 132304

#### Scenario: No frame, no step
- **WHEN** a game of RGSS3 plays that file as music, shows 30 frames and reads `Audio.bgm_pos` thirty times without showing a frame, changing `Graphics.frame_count` between two of the readings
- **THEN** the thirty readings are equal

#### Scenario: Frames of a modal message
- **WHEN** a game of RGSS3 plays that file as music, shows 60 frames, shows a message box that the host dismisses after 30 frames, and reads `Audio.bgm_pos`
- **THEN** the position is 132304

#### Scenario: After a stop, measured on the original
- **WHEN** a game of RGSS3 plays that file as music, shows 24 frames, evaluates `Audio.bgm_stop` and reads `Audio.bgm_pos` at once and after 18 more frames
- **THEN** both readings are 33076

#### Scenario: During a fade, measured on the original
- **WHEN** a game of RGSS3 plays that file as music, evaluates `Audio.bgm_fade(1000)` and shows 60 frames
- **THEN** `Audio.bgm_pos` is 132304

#### Scenario: Formats without a position, measured on the original
- **WHEN** a game of RGSS3 plays as music a file that begins with the header of a Standard MIDI file, then a file of text, then an empty file, each with the `pos` 100000, and shows 60 frames after each
- **THEN** every call returns `nil`, raises nothing, and `Audio.bgm_pos` is 0 after each

#### Scenario: A file that is no sound after a music, measured on the original
- **WHEN** a game of RGSS3 plays that WAV file as music, shows 60 frames, plays a file of text as music and reads `Audio.bgm_pos`
- **THEN** the position is 0

### Requirement: A music effect holds the position of the music
In a game of RGSS3 `me_play` of a sound whose format is known SHALL hold the position of the
music, and not that of the background sound, for the length of the effect at its pitch,
counted in frames at the frame rate of the call. `me_stop` SHALL end the hold and `me_fade`
SHALL NOT. An effect whose format is not known SHALL hold nothing.

#### Scenario: Two seconds, measured on the original
- **WHEN** a game of RGSS3 at 60 frames per second plays the WAV file of 20 s at 44100 Hz in stereo as music, shows 60 frames, plays a WAV file of 2 s as a music effect and reads `Audio.bgm_pos` after 60, 120 and 180 more frames
- **THEN** the readings are 132304, 132304 and 297684

#### Scenario: The background sound goes on, measured on the original
- **WHEN** a game of RGSS3 plays that file as background sound, shows 60 frames, plays the effect of 2 s and shows 60 more frames
- **THEN** `Audio.bgs_pos` is 297684

#### Scenario: Stopped effect, measured on the original
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays a WAV file of 5 s as a music effect, shows 60 frames, evaluates `Audio.me_stop` and shows 60 more frames
- **THEN** `Audio.bgm_pos` is 297684

#### Scenario: Faded effect, measured on the original
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays the effect of 5 s, shows 60 frames, evaluates `Audio.me_fade(500)` and reads `Audio.bgm_pos` after 180 more frames and after 120 beyond those
- **THEN** the readings are 132304 and 297684

#### Scenario: Half the pitch
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays the effect of 2 s with the pitch 50 and reads `Audio.bgm_pos` after 240 and after 300 more frames
- **THEN** the readings are 132304 and 297684

#### Scenario: An effect whose format is not known
- **WHEN** a game of RGSS3 plays that music, shows 60 frames, plays as a music effect a file that begins with the header of a Standard MIDI file and shows 60 more frames
- **THEN** `Audio.bgm_pos` is 297684

### Requirement: Probe of the format of a sound
`vkxp-core` SHALL read from the bytes of a file, without an interpreter and without decoding
a sample, the sample rate, the channel count and the length in sample frames of a WAV file
of 8-bit or 16-bit PCM and of an Ogg Vorbis file, and of the latter the comments `LOOPSTART`
and `LOOPLENGTH`, whose names count in upper case only. For any other bytes it SHALL give
nothing, and no input SHALL make it panic.

#### Scenario: WAV
- **WHEN** the probe is given a generated WAV file of 16-bit PCM at 44100 Hz in stereo with 882000 sample frames, and one of 8-bit PCM at 22050 Hz in mono with 441000 bytes of data
- **THEN** it gives 44100, 2 and 882000 without tags, and 22050, 1 and 441000 without tags

#### Scenario: WAV that was cut short
- **WHEN** the probe is given a WAV file of 16-bit stereo whose data chunk declares 4000 bytes and of which 1000 bytes of data are there
- **THEN** it gives 250 sample frames

#### Scenario: Ogg Vorbis
- **WHEN** the probe is given generated Ogg pages that hold an identification header of 48000 Hz in stereo, a comment header without the two comments, and a last page with the granule position 960000
- **THEN** it gives 48000, 2 and 960000 without tags

#### Scenario: Tags, measured on the original
- **WHEN** the comment header holds `LOOPSTART=88200` and `LOOPLENGTH=176400`, then only the first, then only the second, then `loopstart=88200` and `looplength=176400`
- **THEN** the probe gives both tags, the start alone, the length alone, and no tag

#### Scenario: Comment header across pages
- **WHEN** the comment header is longer than one page holds and continues on the next page, with `LOOPSTART=5` as its last comment
- **THEN** the probe gives the tag 5

#### Scenario: Values that are no numbers
- **WHEN** the comment header holds `LOOPSTART=abc` and `LOOPLENGTH=`
- **THEN** the probe gives the format without tags

#### Scenario: Other bytes
- **WHEN** the probe is given the header of a Standard MIDI file, the first bytes of an MP3 file, the header of an ASF file, a file of text, no bytes at all, and a WAV file whose format tag is not 1
- **THEN** it gives nothing for each

#### Scenario: Any prefix
- **WHEN** the probe is given every prefix of the generated WAV file's first 64 bytes and of the generated Ogg file, and each of these files with one byte changed at every place of its headers
- **THEN** it returns for each, with a format or with nothing

### Requirement: Position of a streamed sound
The position model of `vkxp-core` SHALL follow the rule measured on RGSS3 for a sound of
more than 529200 bytes of 16-bit PCM: a decoder position that starts at `pos`, advances by
15 chunks at the start and by one chunk per 187.5 ms at pitch 100, faster or slower by the
pitch, and wraps from the loop end to the loop start; the position is the decoder's less 16
chunks, not below 0. Time SHALL be counted in frames at their frame rate, in integers.

#### Scenario: Chunk, measured on the original
- **WHEN** the chunk is computed for 44100 Hz in stereo, 44100 Hz in mono, 22050 Hz in stereo, 22050 Hz in mono and 48000 Hz in stereo
- **THEN** it is 33076, 16538, 16540, 8268 and 36000 bytes

#### Scenario: Start, measured on the original
- **WHEN** a sound of 20 s is started at the `pos` 44100 with each of those five formats, and the one of 44100 Hz in stereo at 100001 and at 2147483647
- **THEN** the positions at once are 11024, 27562, 27560, 35832 and 8100, and 66925 and 2147450571

#### Scenario: Steps at 60 frames per second, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo is started at 0 and stepped by frames of 60 per second, read after 0, 12, 18, 24, 36, 48, 60, 72 and 84 frames
- **THEN** the positions are 0, 0, 0, 33076, 66152, 99228, 132304, 165380 and 198456

#### Scenario: Another frame rate
- **WHEN** the same sound is stepped by frames of 40 per second and read after 8, 15 and 40 frames
- **THEN** the positions are 0, 33076 and 132304

#### Scenario: Pitch, measured on the original
- **WHEN** the same sound is stepped 60 frames of 60 per second at the pitches 1, 50, 100, 150 and 200, and 180 frames at 49, 50 and 453
- **THEN** the positions are 0, 33076, 132304, 231532 and 297684, and 198456, 231532 and 2348396

#### Scenario: End of the file, measured on the original
- **WHEN** a sound of 4 s at 44100 Hz in stereo without tags is started at 0 and read after 75 and after 90 frames of 60 per second
- **THEN** the positions are 165380 and 0

#### Scenario: Both tags, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo with `LOOPSTART` 88200 and `LOOPLENGTH` 176400 is started at 600000 and read at once and after 60 frames of 60 per second, and started at 0 and read after 191 and 192 frames
- **THEN** the positions are 0 and 26704, and 496140 and 0

#### Scenario: Only the start tag, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo with `LOOPSTART` 88200 alone is started at 0 and read after 300 frames of 60 per second
- **THEN** the position is 826900

#### Scenario: A music of the run-time package, measured on the original
- **WHEN** a sound of 3020850 sample frames at 44100 Hz in stereo with `LOOPSTART` 280083 and `LOOPLENGTH` 2739337 is started at 11900000
- **THEN** the position at once is 909576

#### Scenario: Start behind the end, measured on the original
- **WHEN** a sound of 20 s at 44100 Hz in stereo is started at 4000000 and read at once and after 24 frames of 60 per second
- **THEN** the positions are 3966924 and 4033076

#### Scenario: Just above the limit, measured on the original
- **WHEN** a sound of 132741 sample frames at 44100 Hz in stereo, 530964 bytes, is started at 100001
- **THEN** the position at once is 0

### Requirement: Position of a short sound
For a sound of at most 529200 bytes of 16-bit PCM the position model SHALL give `pos`
rounded down to a sample frame, plus the sample frames that the frames shown amount to at
the sound's rate and pitch, wrapped at the length of the sound, in bytes of 16-bit PCM. A
sound without a sample frame SHALL give 0.

#### Scenario: Start, measured on the original
- **WHEN** a sound of 3 s at 44100 Hz in stereo, 529200 bytes, is started at 100001
- **THEN** the position at once is 100000

#### Scenario: Frames
- **WHEN** that sound is started at 100000 and read after 60 frames of 60 per second, and started at 0 and read after 90 and after 180 frames
- **THEN** the positions are 276400, 264600 and 0

#### Scenario: Pitch and frame rate
- **WHEN** that sound is started at 0 with the pitch 50 and read after 40 frames of 40 per second
- **THEN** the position is 88200

#### Scenario: No sample frame
- **WHEN** a sound whose length is 0 is started at 100 and stepped 60 frames
- **THEN** the position is 0
