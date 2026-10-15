# Calibrate only time domains the device lists

## Why

The modernization audit of 2026-10-14 found that the pacer of present timing calls
`vkGetCalibratedTimestampsKHR` with a time domain taken from the swapchain's list
(`vkGetSwapchainTimeDomainPropertiesEXT`), but never asks the device which domains it can
calibrate (`vkGetPhysicalDeviceCalibrateableTimeDomainsKHR`). That breaks
VUID-VkCalibratedTimestampInfoKHR-timeDomain-02354: `timeDomain` must be one of the values
that query returns. A driver that offers a swapchain-local domain it cannot calibrate may
fail the call or return garbage, and the validation layer reports it where it checks it.

The same audit found three small defects in the same area: the comments of
`ffi::PRESENT_STAGES` in `present_timing.rs` sit one line off the values they name; the
debug messenger subscribes to `PERFORMANCE` messages at `ERROR` severity only, so it never
receives one; and `docs/decisions.md` says fonts are found through fontconfig (the code scans
directories with `fontdb`) and calls macOS "a distant maybe".

## What Changes

- The pacer reads the system's monotonic clock with `clock_gettime(CLOCK_MONOTONIC)` itself,
  which is what `VK_TIME_DOMAIN_CLOCK_MONOTONIC_KHR` is defined as, so that domain needs no
  Vulkan call.
- For a domain of the swapchain (swapchain-local, present-stage-local) the device's list of
  calibrateable domains is queried once and the domain is used only when it is listed. A
  swapchain whose domains are neither the monotonic clock nor listed ones is paced as without
  present timing.
- A calibrated time of 0 is taken as a change of the swapchain's clock: the domain is chosen
  again, as when the domains counter changes.
- The comments of `PRESENT_STAGES` name the value on their own line.
- The debug messenger no longer asks for `PERFORMANCE` messages, which it could never receive
  at `ERROR` severity; reporting them is left to later work on best-practice checks.
- `docs/decisions.md` names `fontdb` and its directory scan for fonts, and the platform line
  no longer calls macOS "a distant maybe" but says only Linux is supported now.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vulkan-presentation`: calibrated times come only from domains the device can calibrate.

## Impact

`crates/vkrgss-vk` (`compositor/pacer.rs`, `device.rs` and `device/instance.rs` if the query
belongs with the device's capabilities, `present_timing.rs`, `validation.rs`),
`docs/decisions.md`, `CHANGELOG.md`. No composed pixel changes; pacing changes only on
drivers that offered a domain they cannot calibrate.
