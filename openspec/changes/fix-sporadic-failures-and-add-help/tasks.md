# Tasks

## 1. Sporadic failures

- [ ] 1.1 `a_guest_that_is_copied_sends_the_frames_it_sends_without`: cause found, recorded under "Found" and removed. Verify: 200 consecutive passes of `cargo test -p vkxp-binding --test states --locked` while the workspace suite runs in a loop beside it.
- [ ] 1.2 `the_guest_splits_its_frame_time_when_statistics_are_requested`: the same. Verify: 200 consecutive passes of that test (`VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked --test end_to_end the_guest_splits`) under the same load.

## 2. Help option

- [ ] 2.1 `--help` and `-h`. Verify: the scenarios of `specs/game-launch/spec.md` as tests; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.

## 3. Comments and the notice file

- [ ] 3.1 As in `design.md`. Verify: `git grep -n -i "development machine" -- crates xtask` prints nothing; `cargo test -p xtask --locked` (the link check) passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 three times in a row and `openspec validate fix-sporadic-failures-and-add-help --strict` passes.
