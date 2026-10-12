# Tasks

## 1. MIDI loop

- [x] 1.1 The loop marker as in `design.md`. Verify: scenarios "Marker" and "No marker"; `cargo test -p vkxp-audio --locked` passes.

## 2. WMA

- [x] 2.1 The ASF check and the decoding through the program of `MixerOptions`. Verify: scenarios "Program present" (with a stand-in script the test writes into a temporary directory) and "Program absent"; `cargo test -p vkxp-audio --locked` passes.

## 3. Positions

- [x] 3.1 Position and start-from on `Source`; `remember` and `recall` in the mixer. Verify: the three scenarios of "Remembered positions", one of them also with a generated MIDI file and a SoundFont the test generates or the existing MIDI test uses; `cargo test -p vkxp-audio --locked` passes.
- [x] 3.2 `AudioSink::remember` and `recall`, the test doubles, and the two calls of the session. Verify: scenario "Save, then load"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp-host -p vkxp --locked` passes.

## 4. Documentation

- [ ] 4.1 `docs/compatibility.md`: the sentence about MIDI loop markers and the limit "music and background sound start over after a load" go; WMA is listed with its need of `ffmpeg`. `docs/running.md` (audio, save states) and the list of formats in `README.md` say the same. Verify: the documents no longer say that MIDI loop markers are not followed or that music starts over after a load; the tests of `xtask` that check the documents pass.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate loop-midi-decode-wma-and-resume-music --strict` passes.
