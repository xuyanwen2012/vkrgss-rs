# Tasks

## 1. The lookup order of RGSS3

- [x] 1.1 Before anything changes, keep the lists of tests under `local/` as the implementation notes of `design.md` say. Add the kind for the audio of a game of RGSS3 to `crates/vkxp-vfs/src/files.rs` (decision 10) with its tests; `FileKind::Audio` and every existing test stay as they are. Verify: the seven scenarios of "Extension search"; `cargo test -p vkxp-vfs --locked` passes with every name of the kept list.

## 2. The probe and the position model

- [x] 2.1 `crates/vkxp-core/src/sound.rs` and its line in `lib.rs`: the probe of decision 4, with the writers of WAV files and of Ogg pages as test code. Verify: the eight scenarios of "Probe of the format of a sound"; `cargo test -p vkxp-core --locked` passes.
- [x] 2.2 The model of decision 5 in the same module: the streamed sound, the short sound, the clock in frames. Verify: the scenarios of "Position of a streamed sound" and "Position of a short sound", each number as written; `cargo test -p vkxp-core --locked` passes and `cargo clippy -p vkxp-core --all-targets --locked -- -D warnings` passes (the crate forbids `unsafe`).

## 3. The protocol and the mixer

- [x] 3.1 `crates/vkxp-proto`: the two fields of the play command, the volume command, the version one higher, the paragraph on versions in `lib.rs`, the tests (decision 8). Every literal of the play command in the workspace gains `start: 0` and `music_return_ms: 0`, and the binding sends these for every runtime for now; no other expected value of a test changes. Verify: the three scenarios of "Start, return time and volume command of audio"; `cargo test -p vkxp-proto --locked` passes and `cargo build --workspace --all-targets --locked` succeeds.
- [x] 3.2 `crates/vkxp-audio`: the start position, the volume command, the return of the music, the pitch from 1 up (decision 9). Verify: the scenarios of "Start position of a play command", "Volume command", "Return of the music after a music effect" and "Volume, pitch, stop and fade"; `cargo test -p vkxp-audio -p vkxp-host --locked` passes with every name of the kept lists, and the scenarios of "Remembered positions" pass unedited.

## 4. Audio of RGSS3 in the binding

- [x] 4.1 `audio::define` by runtime (decision 1): the functions of RGSS3 with their counts, kinds of arguments, volume, pitch, start and return time (decision 2), the kind of the lookup (decision 10), `setup_midi`. Tests in the new file `crates/vkxp-binding-rgss3/tests/audio_rgss3.rs`; the scenarios that name RGSS1 or RGSS2 in a shared test file of `crates/vkxp-binding/tests`. Verify: the scenarios of "Functions of Audio in RGSS3" except "Missing file while a music plays", of "Volume, pitch and start of a play command of RGSS3" and "Fields that RGSS3 uses"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists and `git diff main -- crates/vkxp-binding-rgss3/tests/ruby192.allow` is empty.
- [x] 4.2 The model in the guest: the state of the two channels, the step at every frame shown outside a modal message (decision 6), `bgm_pos` and `bgs_pos`, the same music again with the volume command (decision 3), stops and fades (decision 7). Verify: the scenarios of "Position of the music and of the background sound" and "The same music again in RGSS3", and "Missing file while a music plays" of "Functions of Audio in RGSS3"; the test command of 4.1 passes.
- [x] 4.3 The hold of a music effect (decision 7). If `Audio.respond_to?(:__reset__)` is true in a game of RGSS3 on this branch, add a test that after `Audio.__reset__` both positions keep their values over 18 frames and that a `bgm_play` after it starts at its `pos`; if it is false, write under Follow-ups of `design.md` that the test was not added. Verify: the six scenarios of "A music effect holds the position of the music"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked --no-fail-fast` passes twice in a row.

## 5. Documents

- [x] 5.1 The documents as the implementation notes of `design.md` list them: `docs/compatibility.md` (the section on VX Ace: `Audio`, the known differences), `docs/decisions.md` ("Audio of RGSS3" under "VX Ace"), `docs/running.md` ("Audio"), the protocol version in `docs/architecture.md` and `README.md` (a line replaced, none added), `CHANGELOG.md`. Verify: the text names nothing that the code does not do and every known difference of decisions 2, 3, 5, 6 and 7 of `design.md`; the tests of `xtask` that check the documents pass (`cargo test -p xtask --locked`).

## 6. Integration check

- [ ] 6.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-audio-of-rgss3 --strict` passes; every name of the kept lists is listed again; `git diff main -- crates/vkxp-guest crates/vkxp-guest-main crates/vkxp-image crates/vkxp-text crates/vkxp-pixel crates/vkxp-compose-ref third_party` is empty.
