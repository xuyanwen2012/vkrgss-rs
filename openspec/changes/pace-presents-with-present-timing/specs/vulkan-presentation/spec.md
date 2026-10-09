# Spec Delta

## ADDED Requirements

### Requirement: Target presentation times
When the device and surface support presentation at an absolute time through
`VK_EXT_present_timing`, the presenter SHALL give every present a target time: the first from
the swapchain's clock, each following one the previous target plus the frame period, restarted
from the clock when pacing is reset or the target has fallen more than four periods behind.
Without that support the presenter SHALL present as before.

#### Scenario: Schedule
- **WHEN** the schedule is asked for five targets at a period of 25 milliseconds from a first time T
- **THEN** they are T, T + 25 ms, T + 50 ms, T + 75 ms and T + 100 ms, without accumulated rounding error over 100000 frames

#### Scenario: Falling behind
- **WHEN** the clock is more than four periods past the next target
- **THEN** the schedule restarts at the clock's time

#### Scenario: Period change
- **WHEN** the frame rate changes from 40 to 20
- **THEN** the next target is 50 milliseconds after the previous one

#### Scenario: No support
- **WHEN** the device lacks the extension, as the CPU device does
- **THEN** frames are presented as before and the tests of presentation pass unchanged

### Requirement: Presentation statistics
On request the presenter SHALL report, from the extension's feedback when it is in use and
from its own clock otherwise, the intervals between the times at which consecutive frames
became visible, as a summary of count, mean, smallest and largest.

#### Scenario: Summary
- **WHEN** visible times 0, 25, 50 and 80 milliseconds are recorded
- **THEN** the summary is three intervals with mean 26.67, smallest 25 and largest 30 milliseconds
