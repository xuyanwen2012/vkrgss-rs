# Tasks

## 1. Work budget and bounded waits

- [x] 1.1 Measure the largest tile-list entry count of the regression games and set `Limit::FrameWork`; record both numbers in design.md under "As built". Verify: `frame_over_the_work_budget_is_a_limit_error` and the existing limit tests pass.
- [x] 1.2 Bound every wait for a submission; a wait that runs out is a lost device. Verify: `wait_that_runs_out_is_a_lost_device`.

## 2. Lost device

- [x] 2.1 `VkError::DeviceLost` and `is_device_lost()`; a lost compositor refuses later calls without touching the device; `release_surface` lets the surface go when idling fails. Verify: `lost_device_refuses_later_calls` and the presentation lifecycle tests pass.
- [x] 2.2 The session ends with a failure on a lost device, windowed and headless. Verify: `lost_device_ends_the_session_with_a_failure` and the session tests of `vkrgss-host` pass.

## 3. Resources and presentation

- [x] 3.1 `ensure_target` replaces the frame target only after the new one exists. Verify: `failed_target_allocation_keeps_the_previous_frame` and "The newest frame after the frame target was recreated" tests pass.
- [ ] 3.2 The staging buffer shrinks after a run of small uploads. Verify: `staging_buffer_shrinks_after_one_large_upload` and the per-frame buffer tests pass.
- [x] 3.3 A present that shows nothing keeps the frame unshown. Verify: `out_of_date_twice_keeps_the_frame_unshown`.
- [x] 3.4 The descriptor check counts the whole set. Verify: `descriptor_check_counts_the_whole_set`.

## 4. Documents

- [x] 4.1 `docs/decisions.md` or `docs/architecture.md` where they describe waits, device requirements or the previous frame; the doc comments of `render` and `Compositor`; `CHANGELOG.md`. Verify: the tests that check the documents pass.

## 5. Checks

- [x] 5.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0; `VK_LOADER_DRIVERS_SELECT='*radeon*' VKRGSS_TEST_DEVICE=RADV cargo test -p vkrgss-vk --locked` passes; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`; `openspec validate bound-the-compositor-and-survive-device-loss --strict` passes.
