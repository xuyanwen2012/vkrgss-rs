## MODIFIED Requirements

### Requirement: Clean under validation
The Vulkan backend SHALL NOT cause any Vulkan validation error, synchronization hazards
included. Its tests SHALL run with the validation layer enabled when the layer is
installed, with its synchronization validation on when the layer offers
`VK_EXT_layer_settings`, and SHALL fail when a validation error is reported.

#### Scenario: Validation error fails a test
- **WHEN** the validation layer reports an error during a test
- **THEN** that test fails and shows the message

#### Scenario: Resources are released
- **WHEN** a backend that has rendered frames and destroyed bitmaps is dropped
- **THEN** the validation layer reports no leaked object

#### Scenario: Synchronization validation is on
- **WHEN** the tests create an instance with a validation layer that offers `VK_EXT_layer_settings`
- **THEN** the instance is created with the setting `validate_sync` set to true

#### Scenario: A hazard fails a test
- **WHEN** a test records a write to an image followed by a read of it without a barrier between them
- **THEN** the validation layer reports a synchronization hazard and the test that checks this fails as expected

## ADDED Requirements

### Requirement: Submissions wait on a timeline
The backend SHALL submit with `vkQueueSubmit2` and SHALL wait for its submissions on a
timeline semaphore instead of a fence that is reset after each wait; uploads SHALL record
the barriers of all images of one upload in one dependency before and one after the copies.

#### Scenario: No fence reset for submissions
- **WHEN** the source of the backend is read
- **THEN** `vkQueueSubmit` and `vkResetFences` are not called for the submissions of composition, upload and read-back

#### Scenario: Same frames
- **WHEN** the differential tests against the CPU reference run
- **THEN** every frame equals the reference, as before
