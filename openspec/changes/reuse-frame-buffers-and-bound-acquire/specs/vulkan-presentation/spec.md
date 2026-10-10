# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
