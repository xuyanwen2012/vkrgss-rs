# vulkan-presentation Specification

## Purpose
Defines how the Vulkan backend shows composited frames in a window: creating and maintaining a
swapchain for a surface, and scaling the logical screen into the window without altering its
pixels beyond the scaling itself.

## Requirements

### Requirement: Presentation is optional
The Vulkan backend SHALL present to a window surface only when it is created for one, and
SHALL then choose a device that can present to that surface. Created without a surface it SHALL
behave exactly as before and need no window system.

#### Scenario: Headless creation is unchanged
- **WHEN** the backend is created without a surface on a machine with no display connection
- **THEN** it initialises and renders frames as it did before this capability existed

#### Scenario: Device cannot present
- **WHEN** the device named by the caller cannot present to the given surface
- **THEN** creation fails with an error that says so and names the device

### Requirement: Scaling into the window
A presented frame SHALL be scaled by a single factor in both directions, the largest at which
it still fits the window, and centred, with the rest of the window black. A window of zero width
or height SHALL present nothing and report no error.

#### Scenario: Exact fit
- **WHEN** a 640 by 480 frame is presented in a 1280 by 960 window
- **THEN** the image fills the window

#### Scenario: Letterbox
- **WHEN** a 640 by 480 frame is presented in a 1920 by 1080 window
- **THEN** the image is 1440 by 1080, centred horizontally, with black bars of 240 pixels left and right

#### Scenario: Minimised window
- **WHEN** the window has zero width
- **THEN** no frame is presented and no error is raised

### Requirement: Sampling
At a whole-number scale factor each frame pixel SHALL appear as a square block of exactly its
own colour. At other factors the image SHALL be sampled so that the interior of each scaled
pixel keeps its colour and only the borders between pixels are interpolated. No colour
conversion SHALL change the values of the frame's channels on the way to the screen.

#### Scenario: Whole-number factor
- **WHEN** a frame is scaled by 2 into an off-screen target and read back
- **THEN** every frame pixel appears as a 2 by 2 block with identical values

#### Scenario: Fractional factor
- **WHEN** a frame with two flat-coloured halves is scaled by 1.5 and read back
- **THEN** pixels well inside each half have exactly that half's colour

### Requirement: Transition mix
The presenter SHALL be able to show a mix of a held frame and the newest frame with an integer
weight `an` from 0 to 255, computed per channel on the frames' 8-bit values before scaling as
`(old * (256 - an) + new * an) >> 8`, and SHALL show the newest frame unmixed when no
transition is active.

#### Scenario: End points
- **WHEN** the mix transition is inactive, and then active with weight 0
- **THEN** the output equals the held frame and then the newest frame

#### Scenario: Half way
- **WHEN** the held frame is black, the newest is white and the weight is one half
- **THEN** the output is mid-grey to within one step per channel

#### Scenario: Measured samples
- **WHEN** each committed cross-fade sample measured on the original runtime is evaluated with its weight
- **THEN** the output equals the measured value

### Requirement: Swapchain lifetime
The backend SHALL recreate its swapchain when the surface is resized or the swapchain is
reported out of date, and SHALL continue presenting afterwards. Where the device offers
swapchain maintenance the backend SHALL replace the swapchain without idling the device;
without it the backend SHALL still replace it correctly. Destroying the backend SHALL release
the swapchain and every related object.

#### Scenario: Resize
- **WHEN** frames are presented, the surface is resized and more frames are presented
- **THEN** all presents succeed and the validation layer reports no error

#### Scenario: Resize without swapchain maintenance
- **WHEN** the same sequence runs with the extension ignored
- **THEN** all presents succeed and the validation layer reports no error

#### Scenario: Teardown
- **WHEN** a backend that has presented frames is dropped
- **THEN** the validation layer reports no leaked object

### Requirement: Testable without a display
Presentation SHALL be exercisable on a CPU device with a headless surface, and the scaled image
SHALL be renderable into an off-screen target of a chosen size and read back, so that scaling
and mixing can be verified without a window.

#### Scenario: Headless surface
- **WHEN** the backend is created for a headless surface on the CPU device and presents frames
- **THEN** every present succeeds with the validation layer reporting nothing

### Requirement: Masked transition mix
The presenter SHALL be able to mix under a mask bitmap, a threshold `T` from 0 to 255 and a
vagueness: with `L` the mask's luminance at the pixel, the weight of the held frame SHALL be
`w = 0` for `L <= T` and `min(255, (L - T) * 255 / vague)` above it, or for a vagueness of 0
`255` for `L >= T` and 0 below, and the output SHALL be `new` for `w = 0` and
`new + (((old - new) * (w + 1)) >> 8)` otherwise. A mask smaller than the frame SHALL be tiled.

#### Scenario: Dark side first
- **WHEN** the mask is black on the left half and white on the right half, the vagueness is 1 and the progress is 0.5
- **THEN** the left half shows the newest frame and the right half the held frame

#### Scenario: Soft edge
- **WHEN** the mask is a horizontal gradient and the vagueness is 64
- **THEN** between the part showing the newest frame and the part showing the held frame there is a band in which the two are mixed

#### Scenario: End
- **WHEN** the progress is 1
- **THEN** the output equals the newest frame for any mask

#### Scenario: Measured samples
- **WHEN** each committed masked sample measured on the original runtime is evaluated with its threshold and vagueness
- **THEN** the output equals the measured value

#### Scenario: Tiled mask
- **WHEN** a 64 by 48 mask is used on a 640 by 480 frame
- **THEN** the pattern repeats every 64 pixels horizontally and every 48 vertically

### Requirement: Scaling filters
The presenter SHALL scale the frame into its letterbox rectangle with one of three filters:
nearest, which shows for each output pixel the frame pixel under its centre; linear, which
interpolates between the four nearest frame pixels; and sharp bilinear, which equals nearest
except in a band one output pixel wide along each frame pixel boundary, where it interpolates.
The filter `auto` SHALL be nearest when the scale factor is a whole number and sharp bilinear
otherwise.

#### Scenario: Integer scale
- **WHEN** a 4 by 4 frame of distinct colours is presented at scale 3 with `auto`
- **THEN** every output pixel equals one frame pixel exactly, in 3 by 3 blocks

#### Scenario: Sharp bilinear at a fractional scale
- **WHEN** a frame of black and white vertical stripes one pixel wide is presented at scale 2.5 with sharp bilinear
- **THEN** each stripe has output columns that are exactly black or exactly white, and no more than one mixed column between neighbouring stripes

#### Scenario: Linear
- **WHEN** the same frame is presented at scale 2.5 with linear
- **THEN** mixed columns exist that sharp bilinear shows as pure black or white

#### Scenario: Mix unaffected
- **WHEN** a transition mix is presented with each filter at scale 1
- **THEN** the output is the same for all three

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

### Requirement: Presentation statistics
On request the presenter SHALL report, from the extension's feedback when it is in use and
from its own clock otherwise, the intervals between the times at which consecutive frames
became visible, as a summary of count, mean, smallest and largest.

#### Scenario: Summary
- **WHEN** visible times 0, 25, 50 and 80 milliseconds are recorded
- **THEN** the summary is three intervals with mean 26.67, smallest 25 and largest 30 milliseconds

### Requirement: Presentation pass without render pass objects
The presentation pass and the off-screen scaling pass SHALL render with dynamic rendering. The
backend SHALL NOT create render pass or framebuffer objects.

#### Scenario: Resize
- **WHEN** frames are presented, the surface is resized and more frames are presented
- **THEN** all presents succeed, the validation layer reports no error, and no framebuffer was created for the new swapchain

#### Scenario: Scaled frame for a dump
- **WHEN** a scaled frame is read back at each scaling filter
- **THEN** it equals the frame the same test produced before this change

### Requirement: Acquiring an image is bounded
The backend SHALL wait a bounded time for a swapchain image. When none arrives, the present
SHALL report that nothing was shown, without an error, and rendering SHALL go on; a later
present SHALL show the newest frame.

#### Scenario: No image in time
- **WHEN** a frame is rendered and the surface hands out no image within the bound
- **THEN** the present returns without an error and reports that nothing was presented, and the frame can be read back

#### Scenario: Images come back
- **WHEN** two presents found no image and the surface hands out images again
- **THEN** the next present shows the newest frame and the validation layer reports no error

### Requirement: Exact values on sRGB surfaces
On a surface that offers only sRGB formats, where the device supports views of a swapchain
image in another format, the presenter SHALL render through a `UNORM` view of the image so
that the 8-bit values of the frame are stored unchanged. Where the device does not support
that, the presenter SHALL still present, compensating for the format's encoding in the shader.

#### Scenario: sRGB only, views supported
- **WHEN** the surface offers only `B8G8R8A8_SRGB` and the device has the mutable-format extension
- **THEN** the swapchain is created for views in `B8G8R8A8_UNORM`, the pass renders through such a view and the shader applies no transfer function

#### Scenario: sRGB only, views not supported
- **WHEN** the same surface is used on a device without the extension
- **THEN** the presenter uses the sRGB format with the inverse transfer in the shader

#### Scenario: Forced path under validation
- **WHEN** the presenter is made to take the sRGB format on a surface that has one, frames are presented and the surface is resized
- **THEN** all presents succeed and the validation layer reports no error

### Requirement: Last-resort surface format
When a surface offers none of the preferred formats, the presenter SHALL take a format in the
sRGB non-linear colour space if there is one.

#### Scenario: Unknown formats
- **WHEN** the surface offers a wide-gamut format first and a 10-bit format in the sRGB non-linear colour space second
- **THEN** the second is chosen

### Requirement: The newest frame after the frame target was recreated
After the logical size has changed, a presented or scaled frame SHALL show the frame
rendered at the new size, also when the driver gives the new frame image the handle of the
one it replaced, and also when no validation layer is loaded.

#### Scenario: Size changes and back
- **WHEN** frames are rendered at 20 by 18, at 32 by 24 and again at 20 by 18 with no validation layer loaded, and the newest frame is scaled into a target
- **THEN** the target shows the last frame and not one rendered before

### Requirement: Releasing an undrawn image where the driver cannot
After a draw has failed, the backend SHALL give the acquired image back with swapchain
maintenance where the driver implements that for the surface, and SHALL otherwise replace
the swapchain. On a headless surface of a Mesa driver older than 25.3.4 it SHALL replace
the swapchain.

#### Scenario: Decision by driver
- **WHEN** the decision is asked for a headless surface with Mesa 25.2.8, 25.3.3, 25.3.4 and 26.2.4, for a headless surface of another driver, and for a window surface of Mesa 25.2.8
- **THEN** the image is released in all cases but the first two

#### Scenario: Present after a failed draw
- **WHEN** a draw fails and frames are presented afterwards
- **THEN** the presents succeed whichever path was taken

### Requirement: Level of a mask pixel
The level `L` of a mask pixel with the colour (r, g, b) SHALL be
`(5014911 * r + 9841699 * g + 1920606 * b) >> 24`, computed without overflow, whatever the
pixel's alpha.

#### Scenario: Grey
- **WHEN** the level of the pixel (v, v, v) is computed for every v from 0 to 255
- **THEN** it is v

#### Scenario: Measured colours
- **WHEN** each committed `lum` sample measured on the original runtime is presented as a mask pixel
- **THEN** the mask value equals the measured level, also for the colours (5, 231, 0) with 137 and (0, 3, 203) with 24

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

### Requirement: A frame not shown stays unshown
A present that shows nothing SHALL leave the newest frame counted as not yet shown, whatever
the reason, so that the next present that shows it is paced as its first.

#### Scenario: Out of date twice
- **WHEN** both attempts of a present meet an out-of-date swapchain, and the next present shows the frame
- **THEN** the pacing statistics count that frame once, at the second present

### Requirement: The surface is released when idling fails
Releasing the surface SHALL destroy the swapchain and let the surface go also when waiting for
the device to be idle fails, and SHALL then report that failure.

#### Scenario: Idle fails
- **WHEN** waiting for the device to be idle fails while the surface is released
- **THEN** an error is returned and the compositor reports its surface as released
