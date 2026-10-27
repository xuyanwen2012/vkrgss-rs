# Tasks

## 1. Fades and NaN

- [x] 1.1 A fade of no time ends the voice at once; the level of a fade with total 0 is 0; `gain` gives 0 for a volume that is not finite; `render` maps NaN to 0. Verify: `fade_of_zero_then_another_fade_is_silent_not_nan` and the existing fade tests of `crates/vkrgss-audio/tests/mixer.rs` pass.

## 2. The callback does not wait

- [x] 2.1 Reports from sources reach the worker instead of the error lock; replaced and ended voices are dropped on the worker. Verify: `callback_reports_damage_without_the_error_lock` and the tests of `crates/vkrgss-audio/tests/output.rs` pass; the module comment of `output.rs` is true.

## 3. Save states

- [x] 3.1 Remember tells a voice that has not started from one that ended. Verify: `remember_right_after_play_keeps_the_file` and the remembered-position tests pass.
- [x] 3.2 Recall of a position in looping MIDI is reduced into the first pass through the loop. Verify: `recall_of_looping_midi_after_long_play_is_bounded` and the MIDI loop marker tests pass.

## 4. MIDI pitch and worker failures

- [x] 4.1 MIDI at a pitch outside the synthesiser's rates plays, resampled. Verify: `midi_at_pitch_400_and_10_plays`.
- [x] 4.2 A panic in the worker is caught per request, reported once, and later commands play. Verify: `panic_in_prepare_leaves_later_commands_playing`.

## 5. Documents

- [ ] 5.1 `docs/compatibility.md` (MIDI pitch, recall of looping MIDI), `CHANGELOG.md`. Verify: the tests that check the documents pass.

## 6. Checks

- [ ] 6.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate keep-the-audio-worker-sound --strict` passes.
