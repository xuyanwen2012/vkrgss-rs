# Design

## Context

The table "the changes that follow" of `run-rgss3-games-in-a-guest-of-their-own` names this
change as number 4: `bgm_play` and `bgs_play` with `pos`, `bgm_pos`, `bgs_pos`,
`setup_midi`, the lookup order of extensions, the fade-in after an ME; source "API report
6"; core API "`AudioCommand` with a start position (protocol), `vkxp-audio`"; depends on
nothing. `Audio.__reset__` is that change's (its requirement "Reset functions of RGSS3"),
not this one's.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{audio,runtime,graphics,guest}.rs`, `crates/vkxp-proto/src/
  {audio,codec,lib}.rs`, `crates/vkxp-audio/src/{mixer,decode}.rs`,
  `crates/vkxp-host/src/audio.rs`, `crates/vkxp-vfs/src/{files,library_version}.rs`.
- `docs/decisions.md`: "Guest / host split" (the guest is deterministic: time comes from
  frame ticks), "VX Ace", the rows A0 to A3 of "Milestones".
- The measurements of the original runtime of RGSS3, outside the repository:
  `../../ref/tools/oracle/measurements/ace-v1-api-REPORT.md` ("the API report"), sections
  6.1 to 6.5, 4.4, 8 and 9, with its sample files `ace-v1-api-audio-samples.*.txt`,
  `ace-v1-api-audio2-samples.301.txt`, `ace-v1-api-audio3-samples.301.txt` and the fit
  script `ace-v1-api-audio-fit.py`; `../../ref/RGSS301_api.md` ("the method list").

What is there today:

| Place | Today |
|---|---|
| `audio.rs` of the binding | one `Audio` for every runtime: 1 to 3 arguments, volume clamped to 0 to 100, pitch to 50 to 150; music and background sound that repeat name, volume and pitch send nothing; a fade forgets what the channel plays |
| `AudioCommand::Play` | channel, the bytes of the file, volume, pitch |
| the mixer | music and background sound repeat the whole file; a music effect pauses the music, which goes on at once at its volume; the pitch is clamped to 50 to 150 a second time; `Source::start_from` exists, for the save states |
| a save state | the audio sink remembers file and position of the two looping channels by slot (`remember`, `recall`); the guest that is resumed sends its play commands again and the sink places them |
| a headless run, and `--no-audio` | a sink that ignores every command; nothing comes back to the guest in any run |
| `FileKind::Audio` | the name as given, then `mid`, `ogg`, `wav`, `mp3`, `wma` |
| a probe on the guest's side | `vkxp_vfs::LibraryVersion` reads the version record of a DLL from bytes; `vkxp-image` reads picture headers. Nothing reads a sound: decoding is the host's (`symphonia` in `vkxp-audio`), which a guest crate may not depend on |

Facts that decide the design:

1. **Nothing flows from the host's audio to the guest, and nothing may.** The audio device
   runs on its own clock, a headless run has none, and the guest's contract is that what
   it does depends on ticks alone. A position asked of the host would differ between a
   window and a headless run and between two runs.
2. **The original's value is not repeatable either.** Two runs of the original with the
   same input wrote saves that differ in the four bytes of the two copies of the position,
   one chunk apart (API report 4.4). The rule is exact in its values and steps; when a
   step happens depends on real time.
3. **The rule needs four numbers of the file and no sample of it:** sample rate, channels,
   length in sample frames, and the two tags. A WAV file has them in its header; an Ogg
   Vorbis file in its first two packets and in the granule position of its last page.

## Goals / Non-Goals

**Goals:**

- A game of RGSS3 can call every function of `Audio` that the method list has, apart from
  `__reset__`, with the arguments the original accepts and the errors it raises.
- `Audio.bgm_pos` and `Audio.bgs_pos` return the value of the measured rule at a time that
  is the count of frames shown, so that two runs of vkxp with the same input write the
  same save.
- The sound starts where `pos` says, the same music again does not restart, the music
  returns over one second after a music effect.
- RGSS1 and RGSS2 do not change: every existing test passes over both interpreters with
  its expected values unedited (a literal of the play command gains its two new fields,
  with 0 and 0, and nothing else of a test changes).

**Non-Goals:**

- `Audio.__reset__` (the guest change).
- A loop of the sound at `LOOPSTART` and `LOOPLENGTH`. The position follows the tags;
  the mixer repeats the whole file as today. See Follow-ups.
- The position of MIDI, MP3 and WMA beyond the measured 0.
- A save that is byte for byte the original's in the four bytes of the position:
  decision 12 says what can be had.
- The original's wildcard lookup (`name.*`), names with a NUL byte or in UTF-16LE, sounds
  inside `Game.rgss3a` beyond what `GameFiles::read` does today.
- One sound effect per file and frame, a fade that follows real time, the volume set
  again in every frame.

## Decisions

1. **One module, told apart by the runtime of the game.** `audio::define` reads the
   runtime of the game from the guest, as `graphics::define` does. For RGSS1 and RGSS2 it defines what it defines
   today, from the functions of today. For RGSS3 it defines `bgm_play` and `bgs_play` (1
   to 4 arguments), `me_play` and `se_play` (1 to 3), the stops and fades, `bgm_pos`,
   `bgs_pos` and `setup_midi`. `__reset__` is not touched: if the guest change has merged,
   its definition stays as it is. No `cfg` and no check of the interpreter is involved.

2. **Arguments of RGSS3** (API report 6.1).
   - Count: `bgm_play` and `bgs_play` with 0 arguments raise `ArgumentError` with the
     message `wrong number of arguments (1 for 0)` and with 5 `(4 for 5)`; `me_play` and
     `se_play` with 4 `(3 for 4)`. The first number is the nearest count the function
     accepts, the second the count given: the reverse of Ruby's own order. If
     `rb::check_count_rgss3` exists on the branch (`bind-the-values-of-rgss3` adds it,
     with exactly this message), `audio.rs` calls it; if not, `audio.rs` words the
     message itself. `rb.rs` is not edited by this change.
   - `volume`, `pitch`, `pos` and the time of a fade go through `rb::int` as today, which
     over Ruby 1.9.2 gives the measured classes and, for `nil` and for a String, the
     measured messages. `pos` alone accepts `nil`, as 0.
   - Volume `v` becomes the volume of the command: 100 for `v >= 100`, `v` for 1 to 99, 0
     for 0, and 100 for `v < 0` (6.3: a negative volume is full volume).
   - Pitch `p` becomes the pitch of the command: 100 for `p < 1`; 100 when the probe knows
     the sample rate `r` of the file and `r * p / 100`, in integers, is above 200000 (6.3:
     454 at 44100 Hz and 1000 at 22050 Hz played at the normal rate, 453 and 907 fast);
     otherwise `p`, at most 65535. The 200000 was seen through Wine and is the documented
     limit of DirectSound. For a file the probe does not know, the limit of 65535 is
     vkxp's: the original asks DirectShow for `p / 100` and what that does was not
     measured.
   - `pos` becomes the start of the command: 0 for `nil` and for a value below 0, and 0
     for a file the probe does not know (6.2: no effect on MP3 and WMA; MIDI was not
     measured). Otherwise the sample frame `pos / (channels * 2)`, rounded down.
   - The functions return `nil`; a file that is not found raises `Errno::ENOENT` as today
     and changes nothing; a file that is no sound raises nothing (the mixer reports it
     once, as today).

3. **The same music again** (6.2, "replay" samples). In RGSS3 a `bgm_play` or `bgs_play`
   whose name, as given, and whose pitch argument equal those of what the channel plays
   sends no play command and leaves the position where it is, whatever `pos` says. When
   the volume differs it sends the new command `Volume`. After a stop or a fade the
   channel plays nothing for this rule, so the next play starts anew at its `pos`: that is
   what `audio.rs` does today, and what was measured (`bgm_fade(1000)`, two seconds, then
   `bgm_play` of the file read 0; the same during a fade).
   - Known difference: on the original the same file is the same name after the lookup,
     so `s44` and `s44.ogg` are one file; in vkxp they are two and the second restarts.
     Another letter case and a backslash are another file on both.

4. **The probe: `vkxp_core::sound::SoundFormat::probe(&[u8]) -> Option<SoundFormat>`.**
   It lives in `vkxp-core` because it is a rule of the guest's side without an interpreter
   and without file access: it takes the bytes `GameFiles::read` returned. It gives the
   sample rate, the channel count, the length in sample frames and the two tags, or
   nothing.
   - *WAV:* `RIFF`, `WAVE`, a `fmt ` chunk of format 1 (PCM) with 8 or 16 bits, and the
     `data` chunk. Frames are the bytes of the data chunk, at most those the file holds,
     divided by the block size. No tags. Every other format tag or width is not known to
     the probe (only these two were measured).
   - *Ogg Vorbis:* the first packet of the stream is the identification header (`\x01`,
     `vorbis`, version, channels, rate), the second the comment header (`\x03`, `vorbis`,
     the vendor string, the comments). Packets are put together from the segments of the
     pages, so a comment header that crosses a page is read. A comment whose name is
     exactly `LOOPSTART` or `LOOPLENGTH`, in upper case, and whose value is decimal digits
     gives a tag; the first of each name counts; any other value leaves the tag out (not
     measured). Frames are the granule position of the last page that begins in the file
     (found from the end by its capture pattern). Checksums of pages are not checked.
   - Everything else (MIDI, MP3, WMA, text, an empty file, a cut header) gives nothing. A
     zero rate or channel count gives nothing.
   - It is written for this purpose and adds no dependency. Rejected: moving `symphonia`
     to the guest (a decoder in the confined process for four numbers, and a second place
     that decides what a file is); asking the host (fact 1).

5. **The model: `vkxp_core::sound::Position`**, the rule of 6.2 with time counted in
   frames. With `B = rate * channels * 2`, `A = channels * 2`, the chunk
   `C = round_up(B * 3 / 16, A)` (integer division), the length `F = frames * A`, the
   loop start `S = LOOPSTART * A` or 0, the loop end `E = min((LOOPSTART + LOOPLENGTH) *
   A, F)` with `LOOPLENGTH` and `F` without:
   - *Streamed* (`F > 529200`). The decoder stands at `D`, first the `pos` argument `P` as
     given (not rounded). Reading `n` bytes goes on to the next limit (`E` while `D < E`,
     else `F`), jumps to `S` there, and goes on; with `P >= F` reading only adds. A start
     reads 15 chunks at once; then one chunk is read each time the clock below has
     counted a chunk's time. The position is `max(0, D - 16 * C)`.
   - *Short* (`F <= 529200`, where the original uses a buffer of the whole file). The
     position is `(floor(P / A) * A mod F + A * n) mod F`, with `n` the sample frames the
     clock has counted. `F = 0` gives 0.
   - *The clock.* One step is one frame shown. For a streamed sound a step adds `4 *
     pitch` to a counter and reads a chunk each time the counter can give `75 * frame
     rate` back (187.5 ms are `75 / 400` s, so at 60 frames per second and pitch 100 a
     chunk is read every 11.25 frames). For a short sound a step adds `rate * pitch` and
     counts a sample frame for every `100 * frame rate`. The pitch is that of the command
     (decision 2), the frame rate that of the frame. Integers only.
   - *Tags that make no region:* `S >= E` is treated as no tags (not measured).
   - A sound the probe does not know has no model and reads 0.
   - The limit of 529200 is inside the interval the measurement left open: 529200 bytes
     were a whole buffer, 530964 were streamed.

6. **Which frames count.** The clock of a channel takes one step for every frame the
   guest sends and waits a tick for outside a modal message: the frames of
   `Graphics.update`, `wait`, `fadeout`, `fadein` and `transition`. The frames that show
   a message box do not count, as they do not count for `Graphics.frame_count`: how many
   there are is the player's doing. A script that sets `Graphics.frame_count` changes
   nothing of the clock. The step is one call in `graphics.rs` where such a frame is
   sent.
   - Known difference: on the original the position goes on in real time during `sleep`,
     during a message box and while a script computes; in vkxp it stands still there.
   - Fast-forward and a host that falls behind do not reach the guest: the position stays
     that of the frames shown, and the sound that is heard may be elsewhere.

7. **Stops, fades and the music effect in the model** (6.2, 6.3).
   - A stop of a channel (`bgm_stop`, `bgs_stop`, and the stop of all four of a reset)
     freezes the position at its last value. Before the first play it is 0.
   - A fade does not stop the model: the position goes on (measured for two seconds).
   - `me_play` holds the clock of the music, not of the background sound, for the time
     of the effect: `ceil(frames * 100 * frame rate / (rate * pitch))` frames by the
     probe of the effect's file, at the frame rate when `me_play` is called. `me_stop`
     ends the hold; `me_fade` does not (measured: the music came back when the effect's
     own time was over). An effect the probe does not know holds nothing: its length is
     not known to the guest. A music started during the hold begins to count when the
     hold ends.
   - Not followed: after two effects in a row the original's music position began at 0
     again (one sample, not explained).

8. **The protocol.** `AudioCommand::Play` gains `start: u64`, the sample frame of the
   file at which the sound begins, and `music_return_ms: u32`, which matters on a music
   effect: the time over which the music returns to its volume when the effect has ended.
   A new command `Volume { channel, volume }` changes the volume of what the channel
   plays. `pitch` is documented as 1 to 65535. `PROTOCOL_VERSION` rises by one: one above
   the version before this change, whichever that is when the flow starts
   (`draw-sprites-and-snapshots-as-rgss3` raises it too). The paragraph on versions of
   `vkxp-proto`, `README.md` and `docs/architecture.md` name the new number; `README.md`
   holds 200 lines and may not grow, so its line is replaced.
   - The binding of RGSS1 and RGSS2 sends `start: 0` and `music_return_ms: 0` and never
     a volume command; that of RGSS3 sends `music_return_ms: 1000` with every music
     effect.
   - The host learns nothing of the runtime: the fields are plain data.

9. **The mixer.**
   - *Start.* A source can be placed at a sample frame of its file: for sampled data the
     output position is that frame scaled by the two rates, reached through
     `Source::start_from`; a MIDI source and one decoded by `ffmpeg` ignore it. A start at
     or behind the end leaves the source at its end, from where a looping channel begins
     again. The loader applies the start where it applies a recalled position today, and
     a recalled position wins: a save state knows where the sound was better than the
     command the resumed guest repeats.
   - *Return of the music.* When a music effect that held the music ends, by itself or by
     a stop, and its command carried a return time above 0, the volume number of the
     music runs evenly from 0 to its value over that time, through the gain curve, as a
     fade runs down (6.3: 1 s, linear in the volume number). A stop of the effect channel
     while no effect holds the music starts nothing. A return time of 0 is today's
     behaviour.
   - *Volume.* The volume command sets the volume of the voice of a channel and restarts
     nothing; without a voice it does nothing.
   - *Pitch.* The mixer's own clamp becomes 1 to 65535. RGSS1 and RGSS2 keep their limits
     of 50 to 150 in the binding, where they are today as well.

10. **The lookup order.** `FileKind` gains a kind for the audio of a game of RGSS3, with
    `ogg`, `mid`, `mp3`, `wav`, `wma`; the binding of RGSS3 asks for it, in `audio.rs` and
    where a resumed copy reads its music again. `FileKind::Audio` keeps its order. The
    tail `mid`, `mp3`, `wav`, `wma` is taken as a fixed list: the measurement cannot tell
    it from the alphabetical order of the directory listing (API report 8, point 5), and
    a list does not depend on the file system. The name as given is still tried first.

11. **A save state.** The model is memory of the guest, so a copy has it and a loaded
    state goes on from the position it was saved with. The play commands a resumed copy
    sends carry `start: 0`; the sink's recalled position places the sound, as for the
    other runtimes.

12. **What a save of a game holds.** The standard scripts store `Audio.bgm_pos` at the
    save, twice. In vkxp that is, for a streamed Ogg or WAV file, `max(0, D - 16 * C)`
    with `D` after as many chunks as the frames shown since the `bgm_play` that started
    the music give at their frame rates and its pitch, less the frames a music effect
    held. It is a value of the original's own sequence for that file and start. It is
    the original's value at that frame when the original has read the same number of
    chunks, which it has when it ran in real time and the save does not fall within a few
    tens of milliseconds after a chunk boundary (the s44 readings at 60 frames per second
    agree at every sixth frame up to frame 84 and differ at frame 90, 1 ms after a
    boundary). The original's two runs of 4.4 differ from each other by one chunk. So the
    comparison of a save for A3 either finds the four bytes equal or finds a difference
    of whole chunks; the oracle change decides how that is recorded, see Follow-ups. For
    a short sound the value is the model's and not the original's cursor; for MIDI, MP3
    and WMA it is 0.

13. **Tests.**
    - `vkxp-core`: the probe on bytes made by the tests (a WAV writer, and an Ogg writer
      that lays out pages with the two Vorbis headers and a last page with a granule
      position; no audio packet is needed) and the model against the numbers of the
      scenarios, which are readings of the report where a scenario says so.
    - `vkxp-audio`: the start on a ramp, as the test of the recalled position does; the
      return of the music and the volume command by amplitudes.
    - The binding: `crates/vkxp-binding-rgss3/tests/audio_rgss3.rs`, a file of that
      package alone, which starts the harness with `testing::start_as(Runtime::Rgss3)`:
      this is how a test of RGSS3 is kept from running as RGSS1 or RGSS2 over Ruby 1.8.7
      (`strings.rs`, `preparation.rs` and `rgss3_script.rs` are such files; the test of
      `xtask` asks for a counterpart of every file of `vkxp-binding`, not the reverse).
      Frames are shown with `testing::play` and a scripted host. The sounds are WAV files
      of silence the test writes into the game directory of the harness.
    - The method surface is checked in that file against the list written out in it
      (`Audio.singleton_methods` without `__reset__`), not against a shared data file.
    - No entry of `ruby192.allow` concerns `Audio`; the list is not edited.

## Implementation notes

- A flow reads `../../ref/` and copies nothing but rules and numbers. No Wine and no
  original runtime is run.
- Before anything changes, keep under `local/` the lists of `cargo test --locked --
  --list` for `vkxp-binding`, `vkxp-binding-rgss3`, `vkxp-proto`, `vkxp-audio`,
  `vkxp-host`, `vkxp-vfs` and `vkxp-core`. At the end every name is there again.
- Where the state of the model lives: a value of `audio.rs` held by `Guest` (one new
  field), with the two channels and the hold of the music effect. `runtime::Playing`
  keeps its three fields; the rule of decision 3 compares its name and pitch.
- `connect` of `runtime.rs` forgets what the looping channels play; it forgets the model
  with them.
- The check of the count in RGSS3 comes before the name is read, and the name before the
  numbers, as today.
- Every command that reaches Vulkan has `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- Documents: `docs/compatibility.md`, section "RPG Maker VX Ace" (created in the style of
  the section on VX if it is not there): what of `Audio` works, and the known
  differences of decisions 2, 3, 5, 6, 7 and of the Non-Goals. `docs/decisions.md`, under
  "VX Ace": a paragraph "Audio of RGSS3" with facts 1 and 2 and decisions 4, 5, 6 and 12
  in the document's own words. `docs/running.md`, "Audio": the start position, the return
  of the music, and that the position a script reads follows frames. `CHANGELOG.md` under
  "Unreleased" (created if absent).

## Risks

- Two flows raise the protocol version at once. The requirement says "one above the
  version before this change", and the number in the documents is that of the branch the
  flow started from; the second merge edits one number.
- A real Ogg file whose last page has no granule position (-1) or whose stream is chained.
  The probe then gives a length that is not the decoder's; the position is still
  deterministic. Recorded under Follow-ups.
- The frame rate of a game that changes it during a hold of a music effect: the hold was
  counted at the old rate.

## Follow-ups

### Left out of this change, with the measured source

Sections are those of the API report.

- A loop of the sound at the tags, so that what is heard after the first loop is where
  the position says: 6.2 (`LOOPSTART`, `LOOPLENGTH`, a start behind the loop end plays to
  the end of the file first). The mixer rewinds by opening the file again in the audio
  callback; a jump to the middle of a file needs a seek there first.
- MIDI positions: every reading was 0, and Wine's `IDirectMusicSegmentState::GetSeek` is
  a stub, so the original's value is not measured (6.2, 6.5; 8, point 7).
- The 48 readings of the fit that are up to a second late, and the lag at a pitch above
  100 (pitch 151 and 200 over 180 frames read one and two chunks less than the rule's
  time gives): 6.2. The model has the rule's time.
- The exact size from which a sound is streamed (between 529200 and 530964 bytes); a
  short sound's cursor, which under Wine moves in steps of 10 ms of real time; two music
  effects in a row; a start position behind the end of the file (one file): 6.2, 6.3; 8,
  point 6.
- When the position of a music that was started during a music effect begins to move
  (the samples show 0 until the effect's end and a value at once after it): 6.3.
- The same file by its name after the lookup: 6.2.
- The lookup `name.*` before the name itself, which also finds `dots.v1.ogg` for `dots`
  and takes the first match when no extension is one of audio; whether the tail of the
  order is a list; a name cut at a NUL byte; a name in UTF-16LE; sounds in
  `Game.rgss3a`: 6.1, 6.4; 8, point 5.
- One buffer for 12 calls of `se_play` of one file in a frame; the volume of a fade
  computed from real time and set in every frame, and a faded sound that is not stopped
  (the mixer ends the voice when its fade ends, the model goes on): 6.3.
- A pitch of MP3 and WMA (`SetRate(p / 100)`) and of MIDI beyond the request: 6.3.
- WAV files that are not PCM of 8 or 16 bits: not measured; the probe does not know them,
  so they read 0 and start at their beginning.
- `Audio.__reset__` with the model: the stop of all four channels freezes both positions
  by decision 7 (measured: 6.2, "reset" samples). If the function exists when this change
  is implemented, task 4.3 tests it; if not, the test comes with whichever change merges
  second.
- For `run-the-oracle-on-vx-ace-games`: a save is compared with the four bytes of the two
  positions either equal or apart by whole chunks of the playing file, and the record of
  A3 says which (4.4, decision 12).
- The message of the `RangeError` for a value of 2**31: the original says ``bignum too
  big to convert into `long'``, a 64-bit guest another text (the decision on the word
  size in `docs/decisions.md`).
