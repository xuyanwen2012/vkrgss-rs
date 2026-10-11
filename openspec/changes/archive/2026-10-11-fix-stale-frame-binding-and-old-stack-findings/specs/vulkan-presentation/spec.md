## ADDED Requirements

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
