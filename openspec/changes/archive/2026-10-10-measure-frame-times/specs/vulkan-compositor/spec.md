# Spec Delta

## ADDED Requirements

### Requirement: Frame timings on request
On request the backend SHALL measure each frame: the processor time of encoding, binning,
uploading, waiting for the submission and presenting, and, on a device with timestamps, the
device time of the compose dispatch and of the presentation pass. Without the request it SHALL
measure nothing. Measuring SHALL NOT change the frame.

#### Scenario: Timings of a frame
- **WHEN** timing is on and a frame with a bitmap upload is rendered
- **THEN** a timing is available whose processor stages are each present, and on a device with timestamps the compose duration is present

#### Scenario: Off by default
- **WHEN** a frame is rendered without the request
- **THEN** no timing is available

#### Scenario: Same frame
- **WHEN** the differential test frames are rendered with timing on
- **THEN** each equals the reference and the validation layer reports no error

### Requirement: Named objects
When the debug-utils extension is available the backend SHALL name its long-lived objects and
label the upload, compose and presentation parts of its command buffers. Without the extension
it SHALL work unchanged.

#### Scenario: Names under validation
- **WHEN** the tests run with the validation layer
- **THEN** naming and labelling produce no validation message
