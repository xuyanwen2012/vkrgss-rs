## MODIFIED Requirements

### Requirement: Target presentation times
Unless pacing by present timing is switched off, when the device and surface support
presentation at an absolute time through `VK_EXT_present_timing` the presenter SHALL give every present a target time: the first from
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

#### Scenario: Switched off
- **WHEN** the environment variable `VKRGSS_PRESENT_TIMING` is `0`
- **THEN** frames are presented as before, whatever the device supports

#### Scenario: No support
- **WHEN** the device lacks the extension, as the CPU device does
- **THEN** frames are presented as before and the tests of presentation pass unchanged

### Requirement: Frozen picture in the buffer of RGSS3
The presenter SHALL keep a buffer of 640 by 480 texels, opaque black until written. A hold
by the buffer rule SHALL write what is shown of the screen, at the newest frame's size, into
its top left corner and leave the rest. While the buffer is the held picture, texel
(x, y) of a newest frame that fits into it SHALL be mixed with buffer texel (x, y) by
"Transition mix" and "Masked transition mix" before scaling. A larger frame SHALL be held
and shown as by the frame rule.

#### Scenario: Larger screen from a fresh buffer, measured on the original
- **WHEN** a frame of 544 by 416 of level 200 is held by the buffer rule, a frame of 640 by 480 of level 100 is rendered and the mix is shown at the weight 127
- **THEN** the shown picture is 640 by 480, with level 150 where x is below 544 and y below 416 and level 49 elsewhere

#### Scenario: The rest of the buffer stays, measured on the original
- **WHEN** a frame of 640 by 480 of level 60 is held by the buffer rule, then a frame of 544 by 416 of level 200 is rendered and held by the buffer rule, a frame of 640 by 480 of level 100 is rendered and the mix is shown at the weight 127
- **THEN** the level is 150 where x is below 544 and y below 416 and 79 elsewhere

#### Scenario: Smaller screen, measured on the original
- **WHEN** a frame of 640 by 480 that has level 200 where x is below 160 and level 60 elsewhere is held by the buffer rule, a frame of 320 by 240 of level 100 is rendered and the mix is shown at the weight 127
- **THEN** the shown picture is 320 by 240, with level 150 where x is below 160 and 79 elsewhere

#### Scenario: Shown frames do not enter the buffer, measured on the original
- **WHEN** a frame of 640 by 480 of level 60 is rendered without a hold, then a frame of 544 by 416 of level 200 is rendered and held by the buffer rule, a frame of 640 by 480 of level 100 is rendered and the mix is shown at the weight 127
- **THEN** the level is 49 where x is 544 or more or y is 416 or more

#### Scenario: Held picture alone and newest frame alone
- **WHEN** in the first scenario the mix is the held picture alone, as while frozen, and then the newest frame alone
- **THEN** the first picture has level 200 where x is below 544 and y below 416 and level 0 elsewhere, and the second has level 100 everywhere

#### Scenario: One size
- **WHEN** frames of one size are held and mixed by the buffer rule and by the frame rule
- **THEN** the shown pictures of the two rules are equal byte for byte

#### Scenario: A frame larger than the buffer
- **WHEN** a frame of 800 by 600 is held by the buffer rule
- **THEN** it is held as by the frame rule, the buffer keeps its texels and no error is reported

#### Scenario: Frame rule unchanged
- **WHEN** the tests of `crates/vkrgss-vk/src/tests/presentation.rs` that existed before this change run with holds of the frame rule
- **THEN** they pass with no expected value edited
