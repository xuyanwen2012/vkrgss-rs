# Tasks

## 1. Decoding (`vkxp-audio`)

- [x] 1.1 Add the dependencies with the minimal feature sets and the `Source` abstraction for sampled formats with format detection. Verify: tests for the WAV and the Ogg and MP3 scenarios of `specs/audio-playback/spec.md` at the source level; `cargo xtask deps` reports no violation.
- [x] 1.2 Add the MIDI source with SoundFont loading. Verify: the MIDI scenario, run only when one of the system SoundFont locations exists, and a test that a missing SoundFont yields the documented error.

## 2. Mixer (`vkxp-audio`)

- [x] 2.1 Implement the mixer with the four channels, looping, replacement and the music-effect pause. Verify: tests for the three scenarios of the channels requirement.
- [x] 2.2 Implement volume, pitch, stop and fade. Verify: tests for the two scenarios of that requirement and for volume 0 and 100.
- [x] 2.3 Implement the failure rules. Verify: the unsupported-data scenario.

## 3. Output and host (`vkxp-audio`, `vkxp-host`, `vkxp`)

- [ ] 3.1 Implement `Output` over `cpal` with the worker thread, and the no-device rule. Verify: a test that constructing it with a device-opening function that fails yields a sink which accepts commands and reports once.
- [ ] 3.2 Add `AudioSink` to the session, the null sink for headless runs and the recording sink for tests; remove the "not implemented" report. Verify: tests for the two scenarios of `specs/host-session/spec.md`.
- [ ] 3.3 Add `--no-audio` and `--soundfont` to the executable and a README paragraph. Verify: argument tests for both options and the two scenarios of `specs/game-launch/spec.md`.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate add-audio-output --strict` passes.
