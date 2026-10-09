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
reported out of date, and SHALL continue presenting afterwards. Destroying the backend SHALL
release the swapchain and every related object.

#### Scenario: Resize
- **WHEN** frames are presented, the surface is resized and more frames are presented
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
- **WHEN** the environment variable `VKXP_PRESENT_TIMING` is `0`
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
