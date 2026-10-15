# Tasks

## 1. Time domains

- [x] 1.1 Query the calibrateable domains, read the monotonic clock directly, choose only listed swapchain domains, treat 0 as a clock change; tests of the pacer's stand-in for the four scenarios. Verify: the scenarios of "Calibrated times only from listed domains"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-vk --locked` passes.

## 2. Small corrections

- [x] 2.1 Move the comments of `PRESENT_STAGES` onto the lines of their values; remove `PERFORMANCE` from the messenger's message types with a comment saying why. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss-vk --locked` passes.
- [x] 2.2 `docs/decisions.md`: fonts through `fontdb` and its directory scan (check the code for the exact directories), and the platform line without "a distant maybe"; `CHANGELOG.md` under Unreleased. Verify: the tests of `xtask` that check the documents and their links pass.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate calibrate-only-listed-time-domains --strict` passes.
