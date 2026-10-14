## MODIFIED Requirements

### Requirement: Probe of the format of a sound
`vkrgss-core` SHALL read from the bytes of a file, without an interpreter and without decoding
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
The position model of `vkrgss-core` SHALL follow the rule measured on RGSS3 for a sound of
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

### Requirement: Input of RGSS1 and RGSS2 is unchanged by RGSS3
`Input` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: marking shown
frames SHALL change nothing there, and every test of the two binding packages that passed
before SHALL pass under its name with its expected values unedited, over both interpreters.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding` and of `vkrgss-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS2 gets a snapshot with C held before it does anything and evaluates `Input.update; [Input::DOWN, Input::F9, Input.press?(Input::C)]`
- **THEN** the result is `[2, 29, true]`, over both interpreters
