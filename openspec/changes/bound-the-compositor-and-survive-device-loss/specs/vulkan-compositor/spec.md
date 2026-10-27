## ADDED Requirements

### Requirement: The work of a frame is bounded
The backend SHALL refuse with a limit error, before it allocates anything for the frame, a frame
whose ops cover more tiles in total than a budget of the backend. The budget SHALL be well above
what the regression games need. Every wait for a submission SHALL end within a bounded time.

#### Scenario: Many full-screen ops at the largest size
- **WHEN** a packet of the largest logical size carries enough full-screen clear ops to exceed the budget
- **THEN** the backend returns a limit error naming the frame work, no buffer grew, and a later small packet renders normally

### Requirement: A lost device is told apart
A lost device, reported by a Vulkan call or by a wait for a submission that ran out, SHALL be
returned as its own kind of error. After it the backend SHALL return that error for every call
that would use the device and SHALL NOT use the device again.

#### Scenario: Lost during a frame
- **WHEN** a call of a frame reports the device lost, and another frame is rendered
- **THEN** both return the lost-device error, and the second does not call the device

#### Scenario: Wait that runs out
- **WHEN** the wait for a submission runs out
- **THEN** the frame returns the lost-device error

### Requirement: The previous frame survives a failed resize
When the frame target cannot be allocated at a new logical size, the backend SHALL keep the
previous frame target and the frame it holds.

#### Scenario: Allocation fails
- **WHEN** a frame is rendered at 20 by 18, and the allocation of the frame target fails for a frame at 32 by 24
- **THEN** the second render returns an error and the frame target still holds the first frame

### Requirement: The upload buffer does not stay at its largest
A staging buffer grown for a large upload SHALL be replaced by a smaller one once a run of
frames has needed much less of it.

#### Scenario: One large upload
- **WHEN** one frame uploads a bitmap of 64 MiB or more, followed by a hundred frames with small uploads
- **THEN** the staging buffer is smaller than the large upload and every frame equals the reference

### Requirement: Descriptor limits of the whole set
The device check SHALL count every descriptor of the composition set against the device's limit
of update-after-bind resources per stage, and SHALL list a device that falls short as
unsuitable with the numbers.

#### Scenario: Exactly the bitmap slots
- **WHEN** a device reports update-after-bind resources per stage equal to the number of bitmap slots
- **THEN** the check lists the device as unsuitable and names the limit
