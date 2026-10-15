## ADDED Requirements

### Requirement: Calibrated times only from listed domains
When pacing by present timing, the presenter SHALL read the monotonic clock domain from the
system's monotonic clock and SHALL call `vkGetCalibratedTimestampsKHR` only with a domain that
`vkGetPhysicalDeviceCalibrateableTimeDomainsKHR` returned. Without a domain it can read this
way it SHALL present as without present timing.

#### Scenario: Monotonic clock
- **WHEN** the swapchain offers `VK_TIME_DOMAIN_CLOCK_MONOTONIC_KHR`
- **THEN** that domain is chosen and the current time is read without calling `vkGetCalibratedTimestampsKHR`

#### Scenario: Swapchain domain the device cannot calibrate
- **WHEN** the swapchain offers only `VK_TIME_DOMAIN_SWAPCHAIN_LOCAL_EXT` and the device's list of calibrateable domains does not contain it
- **THEN** no domain is chosen and frames are presented as without present timing

#### Scenario: Swapchain domain the device can calibrate
- **WHEN** the swapchain offers only `VK_TIME_DOMAIN_SWAPCHAIN_LOCAL_EXT` and the device lists it
- **THEN** that domain is chosen and read with `vkGetCalibratedTimestampsKHR`

#### Scenario: A time of zero
- **WHEN** reading the current time in the chosen domain gives 0
- **THEN** the domain is chosen again and the schedule restarts from the clock
