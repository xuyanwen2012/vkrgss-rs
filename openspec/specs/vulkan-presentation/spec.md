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
The presenter SHALL be able to show a mix of a held frame and the newest frame with a weight
from 0 (held frame only) to 1 (newest frame only), applied per channel before scaling.

#### Scenario: End points
- **WHEN** the mix weight is 0 and then 1
- **THEN** the output equals the held frame and then the newest frame

#### Scenario: Half way
- **WHEN** the held frame is black, the newest is white and the weight is one half
- **THEN** the output is mid-grey to within one step per channel

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
