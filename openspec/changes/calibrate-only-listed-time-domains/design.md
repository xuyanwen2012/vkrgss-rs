# Design

## Context

`compositor/pacer.rs` chooses a time domain with `choose_time_domain` from the domains the
swapchain offers (monotonic clock first, then swapchain-local, then present-stage-local, else
the first) and reads "now" in it with `vkGetCalibratedTimestampsKHR` in `now()`. The device
enables `VK_KHR_calibrated_timestamps` but nobody calls
`vkGetPhysicalDeviceCalibrateableTimeDomainsKHR`.

## Decisions

- **Monotonic without Vulkan.** The specification defines
  `VK_TIME_DOMAIN_CLOCK_MONOTONIC_KHR` as the values of `clock_gettime(CLOCK_MONOTONIC)`. When
  the chosen domain is that one, `now()` reads the clock directly (`libc` or `std` through a
  small safe wrapper; whatever the crate already depends on). This is the common case on
  Mesa and avoids the call entirely.
- **Other domains only when listed.** The list of calibrateable domains is queried once,
  where the pacer is created or with the device's capabilities, whichever fits the existing
  code with fewer lines. `choose_time_domain` takes that list and skips swapchain domains
  (other than the monotonic clock) that are not on it. When nothing usable is left, the pacer
  reports no present-timing support and the presenter paces as without the extension, as the
  requirement "Target presentation times" already says for missing support.
- **Zero is a clock change.** A value of 0 from `now()` is handled like a changed domains
  counter: the domain is chosen again and the schedule restarts from the clock.
- **Testability.** The stand-in of the pacer's tests gains the calibrateable list, so the
  choice is tested without a device.
- **Messenger.** `PERFORMANCE` is removed from the message types. Subscribing to
  performance warnings would need `WARNING` severity and a decision whether they fail tests;
  that belongs to the audit's item on best-practice validation, not here.

## Implementation notes

- The list is queried when the device is opened with present timing and kept in
  `Gpu::calibrateable_domains`; each paced swapchain gets a copy for its choices.
- The monotonic clock is read through `libc`, a workspace dependency that `vkrgss-vk` did
  not list before. Adding it changes one line of `Cargo.lock`, a file outside the scope of
  this change, which the requirement plainly needs.
- A swapchain without a domain that can be read keeps its `SwapchainTiming` (the domains may
  change later), and the pacer plans none of its presents: they go out as without the
  extension, and nothing is reported as a failure.

## Out of scope

Reading swapchain timing properties, VRR reporting, best-practice validation, other audit
items.

## Follow-ups

None yet.
